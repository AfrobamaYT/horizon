//! Generation-owned prepared model geometry and its one-instance pipeline.
use super::{EntityPipelineOptions, entity_pipeline_factory, entity_shader};
use crate::Result;
use glam::{DVec3, Mat3, Mat4, Vec3};
use wgpu::util::DeviceExt;

/// Local corner of a prepared face. Atlas UVs are already resolved.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PreparedModelVertex {
    pub pos: [f32; 4],
    pub uv: [f32; 4],
    pub normal: [f32; 4],
    /// x is 1 when the face takes the instance tint; y is the face emission level.
    pub flags: [u32; 4],
}

/// One draw of a prepared model. The translation is `origin - camera` in f64, then f32.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PreparedModelInstance {
    pub model: [[f32; 4]; 4],
    pub normal: [[f32; 4]; 3],
    pub tint: [f32; 4],
    pub light: u32,
    pub _pad: [u32; 3],
}

impl PreparedModelInstance {
    /// `local` has no camera translation. The camera-relative matrix is what CPU emission uses.
    pub fn new(local: Mat4, origin: DVec3, camera: DVec3, tint: [f32; 4], light: u32) -> Self {
        let model = camera_matrix(local, origin, camera);
        let normal = Mat3::from_mat4(model).inverse().transpose();
        let cols = normal.to_cols_array_2d();
        let mut rows = [[0.0; 4]; 3];
        for column in 0..3 {
            rows[column][..3].copy_from_slice(&cols[column]);
        }
        Self {
            model: model.to_cols_array_2d(),
            normal: rows,
            tint,
            light,
            _pad: [0; 3],
        }
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_cols_array_2d(&self.model)
    }
}

/// Camera-relative pose: subtract in f64, then convert the translation to f32.
pub fn camera_matrix(local: Mat4, origin: DVec3, camera: DVec3) -> Mat4 {
    let delta = origin - camera;
    Mat4::from_translation(Vec3::new(delta.x as f32, delta.y as f32, delta.z as f32)) * local
}

fn shader() -> String {
    entity_shader() + include_str!("../prepared_model.wgsl")
}

const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
    0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Uint32x4
];
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
    4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4,
    8 => Float32x4, 9 => Float32x4, 10 => Float32x4, 11 => Float32x4, 12 => Uint32
];

/// The geometry uploaded for one resource generation, plus the pipelines that draw one instance.
pub struct PreparedModels {
    generation: u64,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    /// Index start and count for each standalone slot.
    ranges: Vec<Option<(u32, u32)>>,
    pipeline: wgpu::RenderPipeline,
    hdr_pipeline: wgpu::RenderPipeline,
}

impl PreparedModels {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The slot's index range. A different generation is refused rather than drawn.
    pub fn indices(&self, generation: u64, slot: u32) -> Result<(u32, u32)> {
        if generation != self.generation {
            return Err("stale prepared model geometry".into());
        }
        self.ranges
            .get(slot as usize)
            .copied()
            .flatten()
            .ok_or_else(|| "prepared model slot has no geometry".into())
    }

    pub fn create(
        device: &wgpu::Device,
        reuse: &super::reuse::Reuse,
        bindings: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
        generation: u64,
        vertices: &[PreparedModelVertex],
        ranges: Vec<Option<(u32, u32)>>,
    ) -> Result<Self> {
        if take_forced_failure() {
            return Err("prepared model upload failed".into());
        }
        if vertices.is_empty() {
            return Err("prepared model geometry is empty".into());
        }
        let indices: Vec<u32> = (0..vertices.len() as u32 / 4)
            .flat_map(|face| {
                [
                    4 * face,
                    4 * face + 1,
                    4 * face + 2,
                    4 * face,
                    4 * face + 2,
                    4 * face + 3,
                ]
            })
            .collect();
        let buffer = |contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Prepared model geometry"),
                contents,
                usage,
            })
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Prepared model instance"),
            source: wgpu::ShaderSource::Wgsl(shader().into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Prepared model instance"),
            bind_group_layouts: &[Some(bindings)],
            immediate_size: 0,
        });
        let buffers = [
            Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<PreparedModelVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &VERTEX_ATTRIBUTES,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<PreparedModelInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &INSTANCE_ATTRIBUTES,
            }),
        ];
        let kept = reuse.on(device);
        let pipeline = |label, blend, depth_write, bias, fragment, write_mask| {
            entity_pipeline_factory(&kept, &layout, &layout, &shader, &buffers, format)(
                EntityPipelineOptions {
                    label,
                    cull_mode: None,
                    depth_write,
                    compare: wgpu::CompareFunction::GreaterEqual,
                    bias,
                    blend,
                    clouds: false,
                    vertex: "vs_prepared_model",
                    fragment,
                    write_mask,
                },
            )
        };
        Ok(Self {
            generation,
            vertices: buffer(bytemuck::cast_slice(vertices), wgpu::BufferUsages::VERTEX),
            indices: buffer(bytemuck::cast_slice(&indices), wgpu::BufferUsages::INDEX),
            ranges,
            pipeline: pipeline(
                "Prepared model instance",
                wgpu::BlendState::ALPHA_BLENDING,
                true,
                wgpu::DepthBiasState {
                    constant: 128,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
                "fs_entity",
                wgpu::ColorWrites::ALL,
            ),
            hdr_pipeline: pipeline(
                "Prepared model HDR mark",
                wgpu::BlendState::REPLACE,
                false,
                wgpu::DepthBiasState {
                    constant: 128,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
                "fs_hdr_mark",
                wgpu::ColorWrites::ALPHA,
            ),
        })
    }

    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        hdr: bool,
        instance: &'a wgpu::Buffer,
        range: (u32, u32),
    ) {
        pass.set_pipeline(if hdr {
            &self.hdr_pipeline
        } else {
            &self.pipeline
        });
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, instance.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(range.0..range.0 + range.1, 0, 0..1);
    }
}

fn take_forced_failure() -> bool {
    FAIL_UPLOAD.swap(false, std::sync::atomic::Ordering::Relaxed)
}

static FAIL_UPLOAD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The next [`PreparedModels::create`] returns an error and uploads nothing.
#[cfg(feature = "test-support")]
pub fn fail_next_prepared_upload() {
    FAIL_UPLOAD.store(true, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_translation_keeps_a_fraction_past_f32_precision() {
        let local = Mat4::from_scale(Vec3::new(1.2, 0.5, 1.7));
        let origin = DVec3::new(9_007_199_254.0, 64.0, -9_007_199_254.5);
        let camera = origin + DVec3::new(0.25, -1.5, 2.0);
        let matrix = camera_matrix(local, origin, camera);
        let translation = matrix.w_axis;
        assert!((translation.x + 0.25).abs() < 1.0e-5, "{translation:?}");
        assert!((translation.y - 1.5).abs() < 1.0e-5, "{translation:?}");
        assert!((translation.z + 2.0).abs() < 1.0e-5, "{translation:?}");
        let lost = (origin.as_vec3() - camera.as_vec3()).x;
        assert_eq!(lost, 0.0, "f32 subtraction of this origin is {lost}");
    }
}
