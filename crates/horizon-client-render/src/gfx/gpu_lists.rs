//! Phase 1's solid section draws made on the GPU (gpu_lists.wgsl).
//!
//! Each frame the render thread walked every section phase 1 draws through
//! `mesher::Sides::visible_from` to find the runs of its quads that face the eye,
//! and pushed one draw per run. Here it pushes one record per section instead:
//! its row (where its quads start and how `Sides` sorted them, uploaded with its
//! mesh), the call each of its parts draws in and its camera-relative origin. The
//! GPU finds the same runs and writes their indices, per call (one per pass
//! group and arena page), into the flat draws' index list, drawn with
//! `draw_indexed_indirect`. Calls take room for all the quads of their sections'
//! parts; the GPU counts the indices it wrote.
//!
//! Solid draws write depth and are tested against it, so their order within a
//! call does not change the picture.
use crate::mesher::{Part, Sides};

/// Words per row: first vertex, `Sides::gpu_row`, padding.
const ROW: usize = 24;
/// Most workgroups along one dimension of a dispatch.
const GROUP_LIMIT: u32 = 65535;

/// A part of a section in a record: `Part` as gpu_lists.wgsl names it.
fn part_code(part: Part) -> u32 {
    match part {
        Part::All => 0,
        Part::Opaque => 1,
        Part::Cutout => 2,
    }
}

/// One of the frame's calls: what pass `group` draws of arena `page`, from
/// index `first` with room for `count`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ListCall {
    pub group: u8,
    pub page: u32,
    pub first: u32,
    pub count: u32,
}

pub(super) struct GpuLists {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    /// Per id: its first vertex and `Sides::gpu_row` (`ROW` words).
    rows: wgpu::Buffer,
    row_capacity: u64,
    free_ids: Vec<u32>,
    next_id: u32,
    /// The frame's sections: id, the call and part of each of its parts, its
    /// number of parts; then its origin.
    records: Vec<[u32; 8]>,
    record_buffer: wgpu::Buffer,
    record_capacity: u64,
    calls: Vec<ListCall>,
    /// `calls`' index by page and group in the band of distance the last
    /// section listed is in (`u32::MAX`: none yet).
    call_of: Vec<u32>,
    band: u32,
    call_data: Vec<[u32; 5]>,
    /// Per call `DrawIndexedIndirectArgs`; the GPU adds what it writes to the count.
    call_buffer: wgpu::Buffer,
    call_capacity: u64,
    view: wgpu::Buffer,
    group: Option<(wgpu::BindGroup, u32)>,
    /// The indices the frame's calls take, and the most they may.
    room: u64,
    cap: u64,
}

fn buffer(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        // COPY_SRC: rows move to a larger buffer; tests read calls back.
        usage: usage | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

fn entry(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

impl GpuLists {
    pub fn new(device: &wgpu::Device) -> Self {
        let read = wgpu::BufferBindingType::Storage { read_only: true };
        let write = wgpu::BufferBindingType::Storage { read_only: false };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Section lists"),
            entries: &[
                entry(0, read),
                entry(1, read),
                entry(2, wgpu::BufferBindingType::Uniform),
                entry(3, write),
                entry(4, write),
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Section lists"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_lists.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Section lists"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Section lists"),
                    bind_group_layouts: &[Some(&layout)],
                    ..Default::default()
                }),
            ),
            module: &module,
            entry_point: Some("flatten_sides"),
            compilation_options: Default::default(),
            cache: None,
        });
        let storage = wgpu::BufferUsages::STORAGE;
        let indirect = wgpu::BufferUsages::INDIRECT | storage;
        Self {
            pipeline,
            layout,
            rows: buffer(device, "Section rows", ROW as u64 * 4, storage),
            row_capacity: 1,
            free_ids: Vec::new(),
            next_id: 0,
            records: Vec::new(),
            record_buffer: buffer(device, "Sections listed", 32, storage),
            record_capacity: 1,
            calls: Vec::new(),
            call_of: Vec::new(),
            band: 0,
            call_data: Vec::new(),
            call_buffer: buffer(device, "Section list calls", 20, indirect),
            call_capacity: 1,
            view: buffer(
                device,
                "Section lists' eye",
                16,
                wgpu::BufferUsages::UNIFORM,
            ),
            group: None,
            room: 0,
            cap: 0,
        }
    }

    /// A mesh uploaded from vertex `start` of its page, its solid quads sorted as
    /// `sides` says: its id, whose row is written through `uploads` in `encoder`.
    pub fn add(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        uploads: &mut super::upload::Uploads,
        start: u32,
        sides: &Sides,
    ) -> u32 {
        let id = self.free_ids.pop().unwrap_or_else(|| {
            self.next_id += 1;
            self.next_id - 1
        });
        self.set(device, encoder, uploads, id, start, sides);
        id
    }

    pub fn remove(&mut self, id: u32) {
        self.free_ids.push(id);
    }

    /// Mesh `id` now starts at vertex `start` (moved to another page).
    pub fn set(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        uploads: &mut super::upload::Uploads,
        id: u32,
        start: u32,
        sides: &Sides,
    ) {
        let need = u64::from(id) + 1;
        if need > self.row_capacity {
            let capacity = need.next_power_of_two().max(1024);
            let rows = buffer(
                device,
                "Section rows",
                capacity * ROW as u64 * 4,
                wgpu::BufferUsages::STORAGE,
            );
            // Earlier rows: uploads record their copies in `encoder`, so this one follows them.
            encoder.copy_buffer_to_buffer(
                &self.rows,
                0,
                &rows,
                0,
                self.row_capacity * ROW as u64 * 4,
            );
            self.rows = rows;
            self.row_capacity = capacity;
        }
        let mut row = [0u32; ROW];
        row[0] = start;
        row[1..23].copy_from_slice(&sides.gpu_row());
        uploads.buffer(
            encoder,
            &self.rows,
            u64::from(id) * ROW as u64 * 4,
            bytemuck::cast_slice(&row),
        );
    }

    /// Starts the frame's lists.
    pub fn clear(&mut self, device: &wgpu::Device) {
        self.records.clear();
        self.calls.clear();
        self.call_of.fill(u32::MAX);
        self.band = 0;
        self.room = 0;
        self.group = None;
        // A quarter of the binding they share with the CPU's draws.
        self.cap = device.limits().max_storage_buffer_binding_size / 16;
    }

    /// Section `id`, in `page`, at `origin` from the camera and `distance`
    /// (squared, as bits) from it: each of `parts` (pass group, part) draws the
    /// quads of the part that face the eye. False, listing nothing, when the
    /// frame's lists hold all the room they may: the caller draws it.
    ///
    /// Sections come nearest first. Each band of distance gets calls of its own,
    /// drawn in that order, as the CPU's lists keep them (`page_runs`): within
    /// a call the GPU writes sections in no set order, and cutout quads, which
    /// tile GPUs can't hide before shading, then draw mostly front to back.
    pub fn list(
        &mut self,
        id: u32,
        page: u32,
        distance: u64,
        sides: &Sides,
        parts: &[(u8, Part)],
        origin: [f32; 3],
    ) -> bool {
        let parts = &parts[..parts.len().min(2)];
        let mut counts = [0; 2];
        for (count, &(_, part)) in counts.iter_mut().zip(parts) {
            *count = sides.part_quads(part) * 6;
        }
        let room = u64::from(counts[0]) + u64::from(counts[1]);
        if self.room + room > self.cap {
            return false;
        }
        self.room += room;
        let band = super::page_runs::band(distance) as u32;
        if band != self.band {
            self.band = band;
            self.call_of.fill(u32::MAX);
        }
        let mut record = [id, 0, 0, parts.len() as u32, 0, 0, 0, 0];
        for (k, &(group, part)) in parts.iter().enumerate() {
            let key = page as usize * 2 + usize::from(group.min(1));
            if self.call_of.len() <= key {
                self.call_of.resize(key + 1, u32::MAX);
            }
            if self.call_of[key] == u32::MAX {
                self.call_of[key] = self.calls.len() as u32;
                self.calls.push(ListCall {
                    group,
                    page,
                    first: 0,
                    count: 0,
                });
            }
            let call = self.call_of[key];
            self.calls[call as usize].count += counts[k];
            record[1 + k] = call | part_code(part) << 28;
        }
        for (to, o) in record[4..7].iter_mut().zip(origin) {
            *to = o.to_bits();
        }
        self.records.push(record);
        true
    }

    /// The sections listed this frame.
    pub fn listed(&self) -> usize {
        self.records.len()
    }

    /// The frame's calls, with their rooms once laid out (`rooms`).
    pub fn calls(&self) -> &[ListCall] {
        &self.calls
    }

    /// Lays the calls' rooms out one after the other from the start of the
    /// index list: the indices they take.
    pub fn rooms(&mut self) -> u64 {
        let mut room = 0u64;
        for call in &mut self.calls {
            call.first = room as u32;
            room += u64::from(call.count);
        }
        room
    }

    /// Stages the frame's sections, calls and eye (camera-relative; none: every
    /// quad faces it), and binds them for `dispatch` writing into `indices`.
    /// After `rooms`.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        uploads: &mut super::upload::Uploads,
        indices: &wgpu::Buffer,
        eye: Option<[f32; 3]>,
    ) {
        self.group = None;
        let records = self.records.len() as u64;
        if records == 0 {
            return;
        }
        if records > self.record_capacity {
            self.record_capacity = records.next_power_of_two();
            self.record_buffer = buffer(
                device,
                "Sections listed",
                self.record_capacity * 32,
                wgpu::BufferUsages::STORAGE,
            );
        }
        uploads.buffer(
            encoder,
            &self.record_buffer,
            0,
            bytemuck::cast_slice(&self.records),
        );
        self.call_data.clear();
        self.call_data
            .extend(self.calls.iter().map(|c| [0, 1, c.first, 0, 0]));
        let calls = self.call_data.len() as u64;
        let indirect = wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE;
        if calls > self.call_capacity {
            self.call_capacity = calls.next_power_of_two();
            self.call_buffer = buffer(
                device,
                "Section list calls",
                self.call_capacity * 20,
                indirect,
            );
        }
        uploads.buffer(
            encoder,
            &self.call_buffer,
            0,
            bytemuck::cast_slice(&self.call_data),
        );
        let [x, y, z] = eye.unwrap_or_default();
        let view = [x, y, z, if eye.is_some() { 1.0 } else { 0.0 }];
        uploads.buffer(encoder, &self.view, 0, bytemuck::cast_slice(&view));
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Section lists"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.record_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(records * 32),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.rows.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.view.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.call_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: indices.as_entire_binding(),
                },
            ],
        });
        self.group = Some((group, records as u32));
    }

    /// Writes the frame's lists (after `prepare`).
    pub fn dispatch(&self, pass: &mut wgpu::ComputePass<'_>) {
        let Some((group, records)) = &self.group else {
            return;
        };
        let groups = *records;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.dispatch_workgroups(groups.min(GROUP_LIMIT), groups.div_ceil(GROUP_LIMIT), 1);
    }

    /// Whether `dispatch` writes this frame's lists.
    pub fn prepared(&self) -> bool {
        self.group.is_some()
    }

    /// Draws call `k` with a pipeline, its page's vertices and the index list bound.
    pub fn draw_indexed(&self, pass: &mut wgpu::RenderPass<'_>, k: usize) {
        pass.draw_indexed_indirect(&self.call_buffer, k as u64 * 20);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesher::{Part, Sides};
    use wgpu::util::DeviceExt;

    struct Section {
        sides: Sides,
        origin: [f32; 3],
        start: u32,
        page: u32,
        parts: &'static [(u8, Part)],
    }

    /// Random sorts: some parts empty, grouped or not, split or not, planes in
    /// and out of a section, each listed in one of the three ways it can be.
    fn sections(count: usize) -> Vec<Section> {
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut next = |n: u32| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((seed >> 33) % u64::from(n)) as u32
        };
        (0..count)
            .map(|_| {
                let mut ends = [0; 14];
                let mut end = 0;
                for e in &mut ends {
                    if next(3) > 0 {
                        end += next(40);
                    }
                    *e = end;
                }
                let planes = std::array::from_fn(|_| next(17) as f32 + next(16) as f32 / 16.0);
                Section {
                    sides: Sides::from_parts(ends, next(2) == 0, next(4) > 0, planes),
                    origin: [0, 1, 2].map(|_| next(129) as f32 - 64.0),
                    start: next(10_000) * 4,
                    page: next(3),
                    parts: match next(3) {
                        0 => &[(0, Part::All)],
                        1 => &[(0, Part::Opaque), (1, Part::Cutout)],
                        _ => &[(1, Part::All)],
                    },
                }
            })
            .collect()
    }

    /// The runs the CPU finds for a section's part, as `Gfx` drew them.
    fn cpu_runs(s: &Section, part: Part, eye: Option<[f32; 3]>) -> Vec<(u32, u32)> {
        let mut runs = Vec::new();
        let draw = |first, quads| runs.push((first, quads));
        match eye {
            Some(eye) => {
                let eye = [0, 1, 2].map(|a| eye[a] - s.origin[a]);
                s.sides.visible_from(eye, eye[1], part, draw);
            }
            None => s.sides.every(part, draw),
        }
        runs
    }

    /// Lists `sections` from each of a few eyes; per eye, the lists, the
    /// calls' words and the indices written.
    fn run(sections: &[Section]) -> Option<Vec<(Option<[f32; 3]>, GpuLists, Vec<u32>, Vec<u32>)>> {
        let (device, queue) = device()?;
        let mut results = Vec::new();
        for eye in [
            Some([3.5f32, 70.25, -12.0]),
            Some([-40.0, -3.0, 50.5]),
            None,
        ] {
            let mut lists = GpuLists::new(&device);
            lists.clear(&device);
            let mut rows = vec![0u32; sections.len() * ROW];
            for (id, s) in sections.iter().enumerate() {
                rows[id * ROW] = s.start;
                rows[id * ROW + 1..id * ROW + 23].copy_from_slice(&s.sides.gpu_row());
                // Nearest first, over several bands.
                let distance = (id as f64 * 2.0).powi(2).to_bits();
                assert!(lists.list(id as u32, s.page, distance, &s.sides, s.parts, s.origin));
            }
            let room = lists.rooms();
            let init = |contents: &[u8], usage| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents,
                    usage,
                })
            };
            let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
            lists.rows = init(bytemuck::cast_slice(&rows), storage);
            let indices = init(&vec![0xffu8; room as usize * 4], storage);
            let mut encoder = device.create_command_encoder(&Default::default());
            let mut uploads = super::super::upload::Uploads::new(&device);
            lists.prepare(&device, &mut encoder, &mut uploads, &indices, eye);
            uploads.finish(&encoder);
            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                lists.dispatch(&mut pass);
            }
            queue.submit([encoder.finish()]);
            let read_back = |buffer: &wgpu::Buffer, words: u64| -> Vec<u32> {
                let read = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: words * 4,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let mut encoder = device.create_command_encoder(&Default::default());
                encoder.copy_buffer_to_buffer(buffer, 0, &read, 0, words * 4);
                queue.submit([encoder.finish()]);
                read.slice(..).map_async(wgpu::MapMode::Read, |_| {});
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                bytemuck::cast_slice(&read.slice(..).get_mapped_range().unwrap()).to_vec()
            };
            let calls = read_back(&lists.call_buffer, lists.calls.len() as u64 * 5);
            let words = read_back(&indices, room);
            results.push((eye, lists, calls, words));
        }
        Some(results)
    }

    fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = crate::gfx::test_gpu::instance();
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
            eprintln!("no GPU adapter: skipped");
            return None;
        };
        let device = pollster::block_on(adapter.request_device(&Default::default())).ok();
        if device.is_none() {
            eprintln!("no GPU device: skipped");
        }
        device
    }

    /// The call section `i`'s `k`th part draws in, as listed.
    fn call_of(lists: &GpuLists, i: usize, k: usize) -> usize {
        (lists.records[i][1 + k] & 0x0fff_ffff) as usize
    }

    /// Each band of distance and page gets a pass's calls of its own, in the
    /// order sections come (nearest first), and sections of one share them.
    #[test]
    fn calls_go_by_band_then_page_nearest_first() {
        let Some((device, _)) = device() else {
            return;
        };
        let mut lists = GpuLists::new(&device);
        lists.clear(&device);
        let sides = Sides::unsorted(4);
        let parts = &[(0, Part::Opaque), (1, Part::Cutout)];
        for (page, blocks) in [(1, 10.0), (0, 20.0), (1, 30.0), (1, 100.0), (0, 130.0)] {
            let distance = f64::powi(blocks, 2).to_bits();
            assert!(lists.list(0, page, distance, &sides, parts, [0.0; 3]));
        }
        let calls: Vec<_> = lists
            .calls()
            .iter()
            .map(|c| (c.group, c.page, c.count))
            .collect();
        assert_eq!(
            calls,
            [
                (0, 1, 48),
                (1, 1, 48),
                (0, 0, 24),
                (1, 0, 24),
                (0, 1, 24),
                (1, 1, 24),
                (0, 0, 24),
                (1, 0, 24)
            ]
        );
    }

    /// `flatten_sides` writes, in each call's room, the indices of exactly the
    /// runs `Sides::visible_from` finds for each listed section's parts (in
    /// some order: workgroups take room as they come), and counts them.
    #[test]
    fn flatten_sides_writes_the_runs_the_cpu_finds() {
        let sections = sections(200);
        let Some(results) = run(&sections) else {
            return;
        };
        for (eye, lists, calls, out) in results {
            let mut expected = vec![Vec::new(); lists.calls.len()];
            for (i, s) in sections.iter().enumerate() {
                for (k, &(_, part)) in s.parts.iter().enumerate() {
                    let call = call_of(&lists, i, k);
                    for (first, quads) in cpu_runs(s, part, eye) {
                        for k in 0..quads * 6 {
                            let corner = [0, 1, 2, 0, 2, 3][(k % 6) as usize];
                            expected[call].push(s.start + first * 4 + (k / 6) * 4 + corner);
                        }
                    }
                }
            }
            for (k, call) in lists.calls.iter().enumerate() {
                let count = calls[k * 5] as usize;
                assert_eq!(
                    count,
                    expected[k].len(),
                    "call {k}'s index count, eye {eye:?}"
                );
                assert_eq!(calls[k * 5 + 2], call.first);
                let mut got = out[call.first as usize..call.first as usize + count].to_vec();
                got.sort_unstable();
                expected[k].sort_unstable();
                assert_eq!(got, expected[k], "call {k}'s indices, eye {eye:?}");
            }
        }
    }
}
