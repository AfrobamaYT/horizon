//! How long the renderer's start spends building its pipelines, and how much of that is the
//! shader compiler: `Renderer::new` on a new device, then again on the same device. wgpu's DX12
//! backend keeps every compiled shader for the device's life, so the second build compiles
//! nothing and the difference is the compiler's (FXC or DXC, `WGPU_DX12_COMPILER`). Synthetic
//! textures: no game resources or player data. Prints one JSON line.
//! `WGPU_DX12_COMPILER=fxc cargo run --release -p horizon-client-render --example boot_pipelines`
use horizon_client_render::gfx::{Renderer, panorama::Faces};
use horizon_client_render::graphics::GraphicsApi;
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

/// The renderer's adapter line (`Horizon renderer: <name> (<backend>, <type>)`), from its log.
struct AdapterLog;
static ADAPTER: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
impl log::Log for AdapterLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }
    fn log(&self, record: &log::Record) {
        let line = record.args().to_string();
        if let Some(adapter) = line.strip_prefix("Horizon renderer: ")
            && adapter.contains('(')
        {
            *ADAPTER.lock().unwrap() = adapter.to_owned();
        }
    }
    fn flush(&self) {}
}

#[derive(Default)]
struct Measure {
    done: bool,
}

fn build(
    window: &Arc<Window>,
    previous: Option<&Renderer>,
) -> horizon_client_render::Result<Renderer> {
    let sprite = 16u32;
    let layers = 64u32;
    let sprites = vec![200u8; (sprite * sprite * 4 * layers) as usize];
    let hud = vec![255u8; 256 * 256 * 4];
    let glint = vec![128u8; 64 * 64 * 4];
    let forcefield = vec![64u8; 32 * 32 * 4];
    pollster::block_on(Renderer::new(
        window.clone(),
        true,
        &sprites,
        sprite,
        sprite,
        layers,
        &[],
        &[],
        (&hud, 256, 256),
        (&glint, 64, 64),
        (&glint, 64, 64),
        (&forcefield, 32),
        Arc::new(Faces::colour([20, 30, 40, 255])),
        None,
        previous,
        if cfg!(windows) {
            GraphicsApi::Dx12
        } else {
            GraphicsApi::Auto
        },
        false,
        false,
    ))
}

impl ApplicationHandler for Measure {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.done {
            return;
        }
        self.done = true;
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_visible(false)
                        .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720)),
                )
                .expect("a window"),
        );
        let started = Instant::now();
        let first = build(&window, None).expect("the renderer");
        let first_ms = started.elapsed().as_secs_f64() * 1000.0;
        let started = Instant::now();
        let second = build(&window, Some(&first)).expect("the renderer again");
        let second_ms = started.elapsed().as_secs_f64() * 1000.0;
        drop(second);
        drop(first);
        println!(
            "{{\"adapter\":{:?},\"compiler\":{:?},\"first_ms\":{first_ms:.0},\"again_ms\":{second_ms:.0},\"compile_ms\":{:.0}}}",
            ADAPTER.lock().unwrap().as_str(),
            std::env::var("WGPU_DX12_COMPILER").unwrap_or_else(|_| "auto".into()),
            first_ms - second_ms
        );
        event_loop.exit();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

fn main() {
    log::set_logger(&AdapterLog).expect("one logger");
    log::set_max_level(log::LevelFilter::Info);
    EventLoop::new()
        .expect("an event loop")
        .run_app(&mut Measure::default())
        .expect("the event loop");
}
