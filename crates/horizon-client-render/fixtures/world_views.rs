//! World views (`horizon_client_render::render_slots::world_views`, the hook notes' "World views") through the real
//! `Renderer`, offscreen: a white wall the player's camera faces and a green one behind it that
//! a view from the other side faces, an entity quad between them, drawn without views and with
//! two. The frame's own picture stays as it was; each view comes back at its size with what its
//! own camera sees. Behind the player's camera, a blue wall facing it: mirrored views draw its
//! front and cull its back as an unmirrored view does, mirrored left to right; a clip plane
//! leaves the green wall out of a view behind it, and a clipped view's entities keep their
//! depth order against terrain. Run with throwaway HORIZON_HOME, HORIZON_LAUNCHER_HOME,
//! XDG_DATA_HOME and XDG_CONFIG_HOME, through production source staging:
//! `python3 tools/run_world_views.py <output-dir>`;
//! `--bench` (an optimized build): instead, the time of a frame of rolling land 49 by 49
//! chunks around the camera at 1920 by 1080 without a view, with a 1 by 1 one (the culling it
//! turns off), a 640 by 360 one and a 1920 by 1080 one.
use client::{
    gfx::{Frame, Renderer, ScreenshotRequest},
    graphics::GraphicsApi,
    hud::HudBatch,
    mesher::{SectionMesh, TerrainVertex, Vertex},
    render_slots::{self, Slots, WorldView, WorldViewDrawn},
};
use glam::DVec3;
use horizon_client_render as client;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

/// The HUD's scale: the window's own, no forced Unicode font.
const GUI: client::hud::GuiScale = client::hud::GuiScale {
    requested: 0,
    unicode: false,
};

/// The views the mod below asks for each frame.
static VIEWS: Mutex<Vec<WorldView>> = Mutex::new(Vec::new());
/// Each drawn view's id, size and a copy of its picture on its way back.
static DRAWN: Mutex<Vec<(u64, [u32; 2], wgpu::TextureFormat, wgpu::Buffer)>> =
    Mutex::new(Vec::new());
static DEVICE: Mutex<Option<wgpu::Device>> = Mutex::new(None);
fn ask_views(views: &mut Vec<WorldView>) {
    views.extend(VIEWS.lock().unwrap().iter().copied());
}
fn view_drawn(view: &mut WorldViewDrawn<'_>) {
    {
        let [width, height] = view.size;
        assert_eq!([view.color.width(), view.color.height()], view.size);
        let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = view.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        view.encoder.copy_texture_to_buffer(
            view.color.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            view.color.size(),
        );
        *DEVICE.lock().unwrap() = Some(view.device.clone());
        DRAWN
            .lock()
            .unwrap()
            .push((view.id, view.size, view.color_format, buffer));
    }
}

fn install_mod() {
    render_slots::install(Slots {
        world_views: ask_views,
        world_views_hooked: || true,
        world_view_drawn: view_drawn,
        world_drawn: |_| {},
        hand_drawn: |_| {},
        render_needs: |_| {},
        world_drawn_hooked: || false,
    });
}

/// The views drawn in the last frame, by id: their size and RGBA pixels.
fn drawn_views() -> Vec<(u64, [u32; 2], Vec<[u8; 4]>)> {
    let device = DEVICE.lock().unwrap().clone();
    DRAWN
        .lock()
        .unwrap()
        .drain(..)
        .map(|(id, [width, height], format, buffer)| {
            let slice = buffer.slice(..);
            slice.map_async(wgpu::MapMode::Read, |r| r.unwrap());
            device
                .as_ref()
                .unwrap()
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();
            let bytes = slice.get_mapped_range().unwrap();
            let row = bytes.len() / height as usize;
            let bgra = matches!(
                format,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
            );
            assert!(
                format.block_copy_size(None) == Some(4),
                "an 8-bit frame: {format:?}"
            );
            let pixels: Vec<[u8; 4]> = (0..height as usize)
                .flat_map(|y| {
                    bytes[y * row..y * row + width as usize * 4]
                        .as_chunks::<4>()
                        .0
                })
                .map(|p| {
                    if bgra {
                        [p[2], p[1], p[0], p[3]]
                    } else {
                        [p[0], p[1], p[2], p[3]]
                    }
                })
                .collect();
            drop(bytes);
            buffer.unmap();
            (id, [width, height], pixels)
        })
        .collect()
}

fn read(path: &Path) -> Vec<[u8; 4]> {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut bytes).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    bytes.truncate(info.buffer_size());
    bytes.as_chunks::<4>().0.to_vec()
}

/// Pixels like `want` (within 40 a channel), of an image.
fn count(pixels: &[[u8; 4]], want: [u8; 3]) -> usize {
    pixels
        .iter()
        .filter(|p| (0..3).all(|i| p[i].abs_diff(want[i]) < 40))
        .count()
}

/// A wall across x 0..16, y 0..16 at z = 12 of a section, in `color`, with each of `windings`:
/// `[0, 1, 2, 3]` faces north (-z), `[3, 2, 1, 0]` south.
fn wall(color: u32, windings: &[[usize; 4]]) -> SectionMesh {
    let mut mesh = SectionMesh::default();
    let corners = [
        [0., 0., 12.],
        [0., 16., 12.],
        [16., 16., 12.],
        [16., 0., 12.],
    ];
    let uvs = [[0., 1.], [0., 0.], [1., 0.], [1., 1.]];
    for order in windings {
        for &i in order {
            mesh.solid
                .push(TerrainVertex::new(corners[i], uvs[i], 0, color, 0xf0f0));
        }
    }
    mesh.sort_solid();
    mesh
}

fn fixture(window: Arc<Window>) -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args().nth(1).ok_or("output directory required")?;
    let out = Path::new(&out);
    std::fs::create_dir_all(out)?;
    install_mod();
    let white = [255; 16 * 16 * 4];
    let zero = [0; 4];
    let mut renderer = pollster::block_on(Renderer::new(
        window,
        true,
        &white,
        16,
        16,
        1,
        &[],
        &[],
        (&white, 16, 16),
        (&zero, 1, 1),
        (&zero, 1, 1),
        (&zero, 1),
        Arc::new(client::gfx::panorama::Faces::colour([0, 0, 0, 255])),
        None,
        None,
        GraphicsApi::Vulkan,
        false,
        true,
        &client::graphics::Graphics::default(),
        Default::default(),
        Default::default(),
    ))?;
    if std::env::args().any(|a| a == "--bench") {
        return bench(&mut renderer, out);
    }
    renderer.resize(320, 180);
    renderer.fullbright = true;
    // (keys count sections from the bottom of the world, y -64: 10 is y 96 to 112)
    // white at z 12, which the player's camera at z 0.5 faces; green at z 28 behind it
    let both = [[0, 1, 2, 3], [3, 2, 1, 0]];
    renderer.upload((0, 10, 0), &wall(u32::MAX, &both));
    renderer.upload((0, 10, 1), &wall(0xff00_ff00, &both));
    // blue at z -4 behind the player's camera, facing south (toward it) only
    renderer.upload((0, 10, -1), &wall(0xffff_0000, &[[3, 2, 1, 0]]));
    // entity quads: red between the player's camera and the white wall, at z 6; yellow 0.1
    // behind the green wall (z 27.9) and cyan on its plane (z 28), seen from past it
    let quad = |[x0, y0, x1, y1]: [f32; 4], z: f32, color: u32| {
        [[x0, y0], [x0, y1], [x1, y1], [x1, y0]]
            .into_iter()
            .zip([[0., 1.], [0., 0.], [1., 0.], [1., 1.]])
            .map(move |([x, y], uv)| Vertex {
                pos: [x - 8.0, y - 104.0, z - 0.5],
                uv,
                layer: 0,
                color,
                light: 0xf0,
                normal: [0, 0, 0, 0],
                glint_uv: [0.0; 2],
            })
    };
    let entity: Vec<Vertex> = quad([7.5, 103.5, 8.5, 104.5], 6.0, 0xff40_40ff)
        .chain(quad([4.0, 100.0, 8.0, 104.0], 27.9, 0xff40_ffff))
        .chain(quad([9.0, 105.0, 13.0, 109.0], 28.0, 0xffff_ff40))
        .collect();
    let hud = HudBatch::new([320, 180], GUI);
    let shot = |renderer: &mut Renderer, name: &str, views: Vec<WorldView>| {
        *VIEWS.lock().unwrap() = views;
        let mut frame = fixture_frame(&hud);
        frame.entities = &entity;
        let path = out.join(format!("{name}.png"));
        renderer.screenshot = Some(ScreenshotRequest {
            path: path.clone(),
            size: None,
            scaled: false,
            background: false,
        });
        renderer.render(&frame).unwrap();
        renderer.render(&frame).unwrap();
        // the frames are submitted on the renderer's own thread: their copies first
        renderer.wait_submissions();
        renderer
            .screenshot_result
            .take()
            .expect("frame readback")
            .unwrap()
            .save()
            .unwrap();
        // the second frame's views
        let mut drawn = drawn_views();
        drawn.drain(..drawn.len() / 2);
        (read(&path), drawn)
    };
    let (plain, none) = shot(&mut renderer, "plain", Vec::new());
    assert!(none.is_empty(), "no view asked for, none drawn");
    let wall_px = count(&plain, [255, 255, 255]);
    let entity_px = count(&plain, [255, 64, 64]);
    assert!(
        wall_px > 1000,
        "the player's camera sees the white wall: {wall_px}"
    );
    assert!(entity_px > 50, "and the entity: {entity_px}");
    assert_eq!(
        count(&plain, [0, 255, 0]),
        0,
        "not the green wall behind it"
    );

    // a camera past the green wall looking back (north), and one just behind the entity
    let behind = WorldView::new(
        7,
        DVec3::new(8.0, 104.0, 40.0),
        180.0,
        0.0,
        70.0,
        [200, 120],
    );
    let close = WorldView::new(9, DVec3::new(8.0, 104.0, 9.0), 180.0, 0.0, 70.0, [64, 64]);
    let (with_views, drawn) = shot(&mut renderer, "views", vec![behind, close]);
    assert!(
        with_views == plain,
        "the frame's picture is the same with views drawn"
    );
    let ids: Vec<u64> = drawn.iter().map(|(id, ..)| *id).collect();
    assert_eq!(ids, [7, 9], "each view drawn once, in order");
    let (_, size, far) = &drawn[0];
    write_png(&out.join("view_7.png"), *size, far)?;
    write_png(&out.join("view_9.png"), drawn[1].1, &drawn[1].2)?;
    assert_eq!(*size, [200, 120]);
    assert_eq!(far.len(), 200 * 120);
    let green = count(far, [0, 255, 0]);
    assert!(
        green > far.len() / 4,
        "the view past the green wall sees it: {green}"
    );
    assert_eq!(
        count(far, [255, 255, 255]),
        0,
        "and not the white one behind it"
    );
    let (_, size, near) = &drawn[1];
    assert_eq!(*size, [64, 64]);
    // a block 3 blocks off at 70 degrees: about 17 by 17 of its 64 by 64 pixels
    let red = count(near, [255, 64, 64]);
    assert!(red > 200, "the view behind the entity sees it: {red}");

    // the game culls entities with the camera's frustum, which takes in what is behind the
    // camera while a view is asked for
    let behind_camera = |renderer: &Renderer| {
        renderer
            .camera_frustum(85.0, 0.0, 0.0, glam::Mat4::IDENTITY, 1000.0)
            .intersects(
                glam::Vec3::new(-1.0, -1.0, -12.0),
                glam::Vec3::new(1.0, 1.0, -10.0),
            )
    };
    assert!(
        behind_camera(&renderer),
        "entities behind the camera are kept for the views"
    );
    *VIEWS.lock().unwrap() = Vec::new();
    assert!(!behind_camera(&renderer), "and culled without views");

    // Mirrored views (64 by 64, x 12): looking north at the blue wall's front 4.5 blocks off, it
    // fills the picture; looking south from z -10 at its back, it is culled, mirrored or not, and
    // the mirrored picture is the unmirrored one left to right (the white wall and the entity
    // are to the right of the middle). And with a clip plane at z 20, the view past the green
    // wall sees the white one instead.
    let at = |id, z, yaw| WorldView::new(id, DVec3::new(12.0, 104.0, z), yaw, 0.0, 70.0, [64, 64]);
    let clipped = behind.with_clip_plane([0.0, 0.0, -1.0, 20.0]);
    let mirrors = vec![
        at(11, 0.5, 180.0).with_mirror(),
        at(12, -10.0, 0.0).with_mirror(),
        at(13, -10.0, 0.0),
        clipped,
    ];
    let (_, drawn) = shot(&mut renderer, "mirrors", mirrors);
    let ids: Vec<u64> = drawn.iter().map(|(id, ..)| *id).collect();
    assert_eq!(ids, [11, 12, 13, 7]);
    for (id, size, pixels) in &drawn {
        write_png(&out.join(format!("mirror_view_{id}.png")), *size, pixels)?;
    }
    let blue = [0, 0, 255];
    let front_blue = count(&drawn[0].2, blue);
    assert_eq!(
        front_blue,
        64 * 64,
        "a mirrored view draws the faces toward it"
    );
    let back_blue = [count(&drawn[1].2, blue), count(&drawn[2].2, blue)];
    assert_eq!(back_blue, [0, 0], "and culls the faces away, as unmirrored");
    let (mirrored, plain_back) = (&drawn[1].2, &drawn[2].2);
    let unlike = (0..64 * 64)
        .filter(|i| {
            let (a, b) = (mirrored[*i], plain_back[i / 64 * 64 + 63 - i % 64]);
            (0..3).any(|c| a[c].abs_diff(b[c]) >= 40)
        })
        .count();
    let asymmetric = (0..64 * 64)
        .filter(|i| {
            let (a, b) = (plain_back[*i], plain_back[i / 64 * 64 + 63 - i % 64]);
            (0..3).any(|c| a[c].abs_diff(b[c]) >= 40)
        })
        .count();
    assert!(
        asymmetric > 200,
        "the unmirrored picture isn't symmetric: {asymmetric}"
    );
    assert!(
        unlike < 64,
        "the mirrored picture is it left to right: {unlike} unlike"
    );
    let clip = &drawn[3].2;
    let clip_white = count(clip, [255, 255, 255]);
    assert_eq!(
        count(clip, [0, 255, 0]),
        0,
        "the clip plane leaves the green wall out"
    );
    assert!(
        clip_white > 2000,
        "and the view sees the white wall: {clip_white}"
    );

    // Clipped views' entities: seen from z 76 through a plane 0.05 in front of the
    // camera, where depth changes little with distance, the yellow quad 0.1 behind the green wall
    // stays hidden; the cyan one on the wall's plane shows as an unclipped view shows it, clip and
    // mirror together too.
    let (yellow, cyan) = ([255, 255, 64], [64, 255, 255]);
    let north = |id, z, size| WorldView::new(id, DVec3::new(8.0, 104.0, z), 180.0, 0.0, 70.0, size);
    let entity_views = vec![
        north(21, 76.0, [256, 256]).with_clip_plane([0.0, 0.0, -1.0, 75.95]),
        north(22, 40.0, [64, 64]).with_clip_plane([0.0, 0.0, -1.0, 36.0]),
        north(23, 40.0, [64, 64]),
        north(24, 40.0, [64, 64])
            .with_clip_plane([0.0, 0.0, -1.0, 36.0])
            .with_mirror(),
    ];
    let (_, drawn) = shot(&mut renderer, "clipped_entities", entity_views);
    for (id, size, pixels) in &drawn {
        write_png(&out.join(format!("entity_view_{id}.png")), *size, pixels)?;
    }
    let behind_wall = count(&drawn[0].2, yellow);
    assert!(
        count(&drawn[0].2, [0, 255, 0]) > 2000,
        "the far view sees the green wall"
    );
    assert_eq!(
        behind_wall, 0,
        "an entity behind a wall stays hidden in a clipped view"
    );
    let coplanar = drawn[1..]
        .iter()
        .map(|v| count(&v.2, cyan))
        .collect::<Vec<_>>();
    assert!(
        coplanar[1] > 150,
        "the unclipped view sees the cyan quad: {coplanar:?}"
    );
    assert!(
        coplanar.iter().all(|&c| c.abs_diff(coplanar[1]) < 8),
        "and the clipped ones, mirrored too, as much of it: {coplanar:?}"
    );
    let (clipped, both) = (&drawn[1].2, &drawn[3].2);
    let clip_mirror_unlike = (0..64 * 64)
        .filter(|i| {
            let (a, b) = (both[*i], clipped[i / 64 * 64 + 63 - i % 64]);
            (0..3).any(|c| a[c].abs_diff(b[c]) >= 40)
        })
        .count();
    assert!(
        clip_mirror_unlike < 64,
        "clipped and mirrored is the clipped picture left to right: {clip_mirror_unlike}"
    );

    // a view asked for again at another size and roll comes back at that size
    let turned = behind.with_roll(30.0);
    let mut resized = turned;
    resized.size = [96, 160];
    let (_, drawn) = shot(&mut renderer, "resized", vec![resized]);
    assert_eq!(drawn.len(), 1);
    assert_eq!((drawn[0].1, drawn[0].2.len()), ([96, 160], 96 * 160));

    let (after, drawn) = shot(&mut renderer, "after", Vec::new());
    assert!(
        drawn.is_empty() && after == plain,
        "with the views gone the frame is as before"
    );
    std::fs::write(
        out.join("result.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "wall_pixels": wall_px, "entity_pixels": entity_px, "view_green_pixels": green,
            "view_entity_pixels": red, "frame_unchanged_with_views": true,
            "mirrored_front_pixels": front_blue, "mirrored_back_pixels": back_blue[0],
            "mirrored_unlike_pixels": unlike, "clipped_white_pixels": clip_white,
            "clipped_behind_wall_pixels": behind_wall, "coplanar_pixels": coplanar,
            "clipped_mirrored_unlike_pixels": clip_mirror_unlike,
            "graphics_adapter": client::crash::system_details("fixture").into_iter()
                .find(|(name, _)| name == "Graphics Adapter").map(|(_, detail)| detail),
        }))?,
    )?;
    println!(
        "world view check passed: {}",
        out.join("result.json").display()
    );
    Ok(())
}

fn write_png(path: &Path, [width, height]: [u32; 2], pixels: &[[u8; 4]]) -> std::io::Result<()> {
    let mut encoder = png::Encoder::new(std::fs::File::create(path)?, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(pixels.as_flattened())?;
    Ok(())
}
/// Rolling land: a column of a section's top faces and the sides they show, at y 64 to 80.
fn hills(cx: i32, cz: i32) -> SectionMesh {
    let height = |x: i32, z: i32| {
        let (x, z) = (f64::from(cx * 16 + x), f64::from(cz * 16 + z));
        (8.0 + 4.0 * (x * 0.09).sin() + 3.0 * (z * 0.07 + x * 0.02).cos()).floor() as i32
    };
    let mut mesh = SectionMesh::default();
    let mut quad = |c: [[f32; 3]; 4]| {
        for (p, uv) in c.into_iter().zip([[0., 1.], [0., 0.], [1., 0.], [1., 1.]]) {
            mesh.solid
                .push(TerrainVertex::new(p, uv, 0, 0xff60_a060, 0xf0));
        }
    };
    for x in 0..16 {
        for z in 0..16 {
            let h = height(x, z);
            let (fx, fz, fh) = (x as f32, z as f32, h as f32);
            quad([
                [fx, fh, fz],
                [fx, fh, fz + 1.],
                [fx + 1., fh, fz + 1.],
                [fx + 1., fh, fz],
            ]);
            // the step down to a lower neighbour, toward +x and +z
            for (dx, dz) in [(1, 0), (0, 1)] {
                let n = height(x + dx, z + dz);
                if n < h {
                    let (lo, hi) = (n as f32, fh);
                    if dx == 1 {
                        quad([
                            [fx + 1., lo, fz],
                            [fx + 1., hi, fz],
                            [fx + 1., hi, fz + 1.],
                            [fx + 1., lo, fz + 1.],
                        ]);
                    } else {
                        quad([
                            [fx, lo, fz + 1.],
                            [fx + 1., lo, fz + 1.],
                            [fx + 1., hi, fz + 1.],
                            [fx, hi, fz + 1.],
                        ]);
                    }
                }
            }
        }
    }
    mesh.sort_solid();
    mesh
}

/// `--bench`: milliseconds a frame without a view, with a small one and with a full-size one.
fn bench(renderer: &mut Renderer, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    const R: i32 = 24;
    renderer.resize(1920, 1080);
    let mut sections = 0;
    for cx in -R..=R {
        for cz in -R..=R {
            // (keys count sections from y -64: 8 is y 64 to 80)
            renderer.upload((cx, 8, cz), &hills(cx, cz));
            sections += 1;
        }
    }
    let hud = HudBatch::new([1920, 1080], GUI);
    let mut time = |views: Vec<WorldView>| {
        *VIEWS.lock().unwrap() = views;
        let mut frame = fixture_frame(&hud);
        frame.camera = DVec3::new(8.0, 90.0, 8.0);
        frame.eye = frame.camera;
        frame.pitch = 15.0;
        frame.eye_rotation = [0.0, 15.0];
        frame.distance = 16.0 * R as f32;
        frame.detail_distance = frame.distance;
        frame.depth_far = 2000.0;
        frame.cave_culling = true;
        for _ in 0..30 {
            renderer.render(&frame).unwrap();
            DRAWN.lock().unwrap().clear();
        }
        let started = std::time::Instant::now();
        const FRAMES: u32 = 240;
        for _ in 0..FRAMES {
            renderer.render(&frame).unwrap();
            DRAWN.lock().unwrap().clear();
        }
        // the last frame's picture, so the GPU has caught up
        renderer.screenshot = Some(ScreenshotRequest {
            path: out.join("bench.png"),
            size: None,
            scaled: false,
            background: false,
        });
        renderer.render(&frame).unwrap();
        renderer.render(&frame).unwrap();
        DRAWN.lock().unwrap().clear();
        started.elapsed().as_secs_f64() * 1000.0 / f64::from(FRAMES + 2)
    };
    // a camera 60 blocks away, looking back across the land
    let view = |size| WorldView::new(1, DVec3::new(70.0, 95.0, 40.0), 120.0, 10.0, 70.0, size);
    let plain = time(Vec::new());
    // a 1 by 1 view: what the frame's culling for views costs, nearly without a view's pixels
    let culling = time(vec![view([1, 1])]);
    let small = time(vec![view([640, 360])]);
    let full = time(vec![view([1920, 1080])]);
    let again = time(Vec::new());
    let result = serde_json::json!({
        "sections": sections, "frame_ms_without_view": plain, "frame_ms_with_1x1_view": culling,
        "frame_ms_with_640x360_view": small,
        "frame_ms_with_1920x1080_view": full, "frame_ms_without_view_after": again,
        "graphics_adapter": client::crash::system_details("bench").into_iter()
            .find(|(name, _)| name == "Graphics Adapter").map(|(_, detail)| detail),
    });
    println!("{result}");
    std::fs::write(out.join("bench.json"), serde_json::to_vec_pretty(&result)?)?;
    Ok(())
}

struct App;
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_visible(false))
                .unwrap(),
        );
        if let Err(error) = fixture(window) {
            eprintln!("world view check failed: {error}");
            std::process::exit(1);
        }
        event_loop.exit();
    }
    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}
fn main() {
    for key in [
        "HORIZON_HOME",
        "HORIZON_LAUNCHER_HOME",
        "XDG_DATA_HOME",
        "XDG_CONFIG_HOME",
    ] {
        assert!(
            std::env::var_os(key).is_some(),
            "set throwaway {key} (AGENTS.md)"
        );
    }
    EventLoop::new().unwrap().run_app(&mut App).unwrap();
}

fn fixture_frame(hud: &HudBatch) -> Frame<'_> {
    Frame {
        weather: &[],
        weather_time: client::weather::Time::new(0, 0.0),
        camera: glam::DVec3::new(8.0, 104.0, 0.5),
        yaw: 0.0,
        pitch: 0.0,
        fov_degrees: 85.0,
        eye: glam::DVec3::new(8.0, 104.0, 0.5),
        eye_rotation: [0.0, 0.0],
        first_person: true,
        bob: glam::Mat4::IDENTITY,
        spin: glam::Mat4::IDENTITY,
        sky: client::environment::Sky::default(),
        shadow_scene_revision: (0, 0),
        shadow_cache: false,
        distance: 128.0,
        detail_distance: 128.0,
        depth_far: 1000.0,
        underwater: false,
        water_plane: None,
        cave_culling: false,
        title: false,
        sky_blocked: false,
        skybox: false,
        end_sky: false,
        end_sky_sprite: None,
        boss_world_fog: false,
        environment_fog: None,
        water_fog: [0.0; 3],
        fog: None,
        dark_disc: false,
        outline: Vec::new(),
        outline_after_terrain: false,
        outline_high_contrast: false,
        pvp: Default::default(),
        debug_lines: &[],
        debug_lines_on_top: &[],
        debug_crosshair: &[],
        hud,
        glint_alpha: 0.75,
        glint_ticks: 0.0,
        nether_lighting: false,
        entities: &[],
        static_block_entities: &[],
        visible_static_block_entity_sections: &[],
        static_block_entity_stats: Default::default(),
        entity_motion: &[],
        entity_motion_posed: &[],
        entity_spans: &[],
        skin: None,
        glowing: &[],
        dynamic_lights: &[],
        colored_lights: &[],
        emissive_layers: &[],
        clouds: None,
        lightning: Default::default(),
        additive: &[],
        see_through: &[],
        world_primitives: Default::default(),
        shadow_only: &[],
        celestial: &[],
        stars: None,
        hand: &[],
        hand_items: &[],
        hand_fov_degrees: 70.0,
        screen: &[],
        pack: Default::default(),
        border: None,
        game_time: 0.0,
        shader_game_time: 0.0,
        end_portal_sprites: [[0.0; 4]; 2],
        crumbling_sprite: [0.0; 4],
        ticks: 0,
        darkness: (0.0, 0.0),
        night_vision: 0.0,
        block_flicker: 0.0,
        boss_darken: 0.0,
        chunk_fade: 0.0,
        post_effects: client::post_chain::Request::none(),
        menu_blur: None,
    }
}
