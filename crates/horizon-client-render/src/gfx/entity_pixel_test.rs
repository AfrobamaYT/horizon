//! Offscreen entity pipeline readback for cross-crate rendering tests.
use super::*;
use bytemuck::Zeroable;

const SIZE: u32 = 64;

struct Frame {
    device: wgpu::Device,
    queue: wgpu::Queue,
    color: wgpu::Texture,
    depth: wgpu::Texture,
    group: wgpu::BindGroup,
    bindings: wgpu::BindGroupLayout,
}

fn software_frame(
    atlas_size: [u32; 2],
    atlas: &[u8],
    view_proj: Mat4,
) -> std::result::Result<Frame, String> {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = wgpu::Backends::VULKAN;
    let instance = wgpu::Instance::new(desc);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        force_fallback_adapter: true,
        ..Default::default()
    }))
    .map_err(|error| format!("no software Vulkan adapter: {error}"))?;
    if adapter.get_info().device_type != wgpu::DeviceType::Cpu {
        return Err("no software Vulkan adapter".into());
    }
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
        .map_err(|error| format!("software Vulkan device request failed: {error}"))?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let texture = |width, height, format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = texture(
        SIZE,
        SIZE,
        format,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = texture(
        SIZE,
        SIZE,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let atlas_texture = texture(
        atlas_size[0],
        atlas_size[1],
        format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    );
    queue.write_texture(
        atlas_texture.as_image_copy(),
        atlas,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(atlas_size[0] * 4),
            rows_per_image: None,
        },
        atlas_texture.size(),
    );
    let lightmap = texture(
        16,
        16,
        format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    );
    queue.write_texture(
        lightmap.as_image_copy(),
        &[128, 192, 255, 255].repeat(16 * 16),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(64),
            rows_per_image: None,
        },
        lightmap.size(),
    );
    let sampler = device.create_sampler(&Default::default());
    let mut globals = Globals::zeroed();
    globals.view_proj = view_proj.to_cols_array_2d();
    globals.fog = [1000.0, 2000.0, 1000.0, 2000.0];
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(&globals),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bindings = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &entity_binding_entries(),
    });
    let atlas_view = atlas_texture.create_view(&Default::default());
    let light_view = lightmap.create_view(&Default::default());
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bindings,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&atlas_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&light_view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(&atlas_view),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(&atlas_view),
            },
        ],
    });
    Ok(Frame {
        device,
        queue,
        color,
        depth,
        group,
        bindings,
    })
}

fn readback(frame: &Frame, encoder: wgpu::CommandEncoder) -> std::result::Result<Vec<u8>, String> {
    let readback = frame.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(SIZE * SIZE * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = encoder;
    encoder.copy_texture_to_buffer(
        frame.color.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: None,
            },
        },
        frame.color.size(),
    );
    frame.queue.submit([encoder.finish()]);
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    frame
        .device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|error| error.to_string())?;
    Ok(readback
        .slice(..)
        .get_mapped_range()
        .map_err(|error| error.to_string())?
        .to_vec())
}

/// Renders CPU entity quads. A missing software Vulkan adapter fails the test.
pub fn render(
    vertices: &[Vertex],
    atlas_size: [u32; 2],
    atlas: &[u8],
    view_proj: Mat4,
    hdr: bool,
) -> std::result::Result<Vec<u8>, String> {
    let frame = software_frame(atlas_size, atlas, view_proj)?;
    let shader = frame
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(entity_shader().into()),
        });
    let layout = frame
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&frame.bindings)],
            immediate_size: 0,
        });
    let buffers = [Some(wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ENTITY_ATTRIBUTES,
    })];
    let reuse = reuse::Reuse::default();
    let kept = reuse.on(&frame.device);
    let make = |fragment, depth_write, bias, blend, write_mask| {
        entity_pipeline_factory(
            &kept,
            &layout,
            &layout,
            &shader,
            &buffers,
            wgpu::TextureFormat::Rgba8Unorm,
        )(EntityPipelineOptions {
            label: "Entities",
            cull_mode: None,
            depth_write,
            compare: wgpu::CompareFunction::GreaterEqual,
            bias,
            blend,
            clouds: false,
            vertex: "vs_entity",
            fragment,
            write_mask,
        })
    };
    let pipeline = make(
        "fs_entity",
        true,
        wgpu::DepthBiasState {
            constant: 128,
            slope_scale: 0.0,
            clamp: 0.0,
        },
        wgpu::BlendState::ALPHA_BLENDING,
        wgpu::ColorWrites::ALL,
    );
    let hdr_pipeline = hdr.then(|| {
        make(
            "fs_hdr_mark",
            false,
            wgpu::DepthBiasState {
                constant: 128,
                slope_scale: 0.0,
                clamp: 0.0,
            },
            wgpu::BlendState::REPLACE,
            wgpu::ColorWrites::ALPHA,
        )
    });
    let vertex = frame
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
    let indices: Vec<u32> = (0..vertices.len() as u32 / 4)
        .flat_map(|i| [4 * i, 4 * i + 1, 4 * i + 2, 4 * i, 4 * i + 2, 4 * i + 3])
        .collect();
    let index = frame
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
    let mut encoder = frame.device.create_command_encoder(&Default::default());
    {
        let color_view = frame.color.create_view(&Default::default());
        let depth_view = frame.depth.create_view(&Default::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &frame.group, &[]);
        pass.set_vertex_buffer(0, vertex.slice(..));
        pass.set_index_buffer(index.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        if let Some(hdr_pipeline) = &hdr_pipeline {
            pass.set_pipeline(hdr_pipeline);
            pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }
    }
    readback(&frame, encoder)
}

/// Inputs for one offscreen prepared instance. Fields stay named so the draw is not a long argument list.
pub struct PreparedInstanceDraw<'a> {
    pub vertices: &'a [super::prepared_model::PreparedModelVertex],
    pub ranges: Vec<Option<(u32, u32)>>,
    pub uploaded: u64,
    pub requested: u64,
    pub slot: u32,
    pub instances: &'a [super::prepared_model::PreparedModelInstance],
    /// When set, each copy is its own draw. Otherwise they are one run.
    pub per_copy: bool,
    /// Colour pass, then the HDR mark. Off draws the colour pass only.
    pub hdr: bool,
    pub atlas_size: [u32; 2],
    pub atlas: &'a [u8],
    pub view_proj: Mat4,
}

/// One prepared instance plus its HDR mark. The geometry is dropped before submit.
pub fn render_instance(draw: PreparedInstanceDraw<'_>) -> std::result::Result<Vec<u8>, String> {
    let PreparedInstanceDraw {
        vertices,
        ranges,
        uploaded,
        requested,
        slot,
        instances,
        per_copy,
        hdr,
        atlas_size,
        atlas,
        view_proj,
    } = draw;
    let frame = software_frame(atlas_size, atlas, view_proj)?;
    let models = super::prepared_model::PreparedModels::create(
        &frame.device,
        &reuse::Reuse::default(),
        &frame.bindings,
        wgpu::TextureFormat::Rgba8Unorm,
        uploaded,
        vertices,
        ranges,
    )
    .map_err(|error| error.to_string())?;
    let range = models
        .indices(requested, slot)
        .map_err(|error| error.to_string())?;
    let instance = frame
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(instances),
            usage: wgpu::BufferUsages::VERTEX,
        });
    let mut encoder = frame.device.create_command_encoder(&Default::default());
    let passes = if hdr { 2 } else { 1 };
    for (mark, clear) in [(false, true), (true, false)].into_iter().take(passes) {
        let color_view = frame.color.create_view(&Default::default());
        let depth_view = frame.depth.create_view(&Default::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: if clear {
                        wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: if clear {
                        wgpu::LoadOp::Clear(0.0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_bind_group(0, &frame.group, &[]);
        let runs = if per_copy {
            (0..instances.len() as u32)
                .map(|first| super::prepared_model::InstanceRun { first, count: 1 })
                .collect::<Vec<_>>()
        } else {
            vec![super::prepared_model::InstanceRun {
                first: 0,
                count: instances.len() as u32,
            }]
        };
        for run in runs {
            models.draw_run(&mut pass, mark, &instance, range, run);
        }
    }
    drop(models);
    readback(&frame, encoder)
}

#[cfg(test)]
#[rustfmt::skip]
mod instance_run {
    use super::super::prepared_model::{PreparedModelInstance, PreparedModelVertex};
    use super::{PreparedInstanceDraw, SIZE};
    use glam::{Mat4, Vec3};

    fn corner(x: f32, y: f32) -> PreparedModelVertex {
        PreparedModelVertex { pos: [x, y, 0.5, 1.0], uv: [0.5, 0.5, 0.0, 0.0], normal: [0.0, 0.0, 1.0, 0.0], flags: [1, 0, 0, 0] }
    }

    fn instance_at(x: f32, tint: [f32; 4]) -> PreparedModelInstance {
        let mut normal = [[0.0; 4]; 3];
        normal[0][0] = 1.0;
        normal[1][1] = 1.0;
        normal[2][2] = 1.0;
        PreparedModelInstance { model: Mat4::from_translation(Vec3::new(x, 0.0, 0.0)).to_cols_array_2d(), normal, tint, light: 0xff00, _pad: [0; 3] }
    }

    fn render(per_copy: bool, hdr: bool) -> Result<Vec<u8>, String> {
        let instances = [instance_at(-0.45, [1.0, 0.0, 0.0, 1.0]), instance_at(0.45, [0.0, 0.0, 1.0, 1.0])];
        let vertices = [corner(-0.2, -0.2), corner(0.2, -0.2), corner(0.2, 0.2), corner(-0.2, 0.2)];
        super::render_instance(PreparedInstanceDraw {
            vertices: &vertices, ranges: vec![Some((0, 6))], uploaded: 1, requested: 1, slot: 0,
            instances: &instances, per_copy, hdr, atlas_size: [4, 4],
            atlas: &[255, 255, 255, 255].repeat(16), view_proj: Mat4::IDENTITY,
        })
    }

    #[test]
    #[ignore = "needs software Vulkan"]
    fn an_instanced_run_matches_per_copy_pixels_with_hdr() {
        let per_copy = render(true, true).expect("per-copy software Vulkan draw");
        let instanced = render(false, true).expect("instanced software Vulkan draw");
        assert_eq!(per_copy, instanced);
        assert_eq!(per_copy.len(), (SIZE * SIZE * 4) as usize);
        let covered = per_copy.as_chunks::<4>().0.iter().filter(|p| p[0] != 0 || p[2] != 0).count();
        assert!(covered > 8, "the run covered {covered} pixels");
        let plain = render(false, false).expect("color-only software Vulkan draw");
        let alpha = |pixels: &[u8]| pixels.iter().skip(3).step_by(4).copied().collect::<Vec<_>>();
        assert_ne!(alpha(&per_copy), alpha(&plain), "the HDR mark did not change alpha");
    }
}
