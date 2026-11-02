//! Pipelines kept between the renderers built on one device. The start builds
//! three (the boot screen's, the title artwork's, the world's) and a resource
//! reload or a graphics option builds another; each compiled every shader
//! module and pipeline again, though most come out identical (the boot and
//! title renderers' entirely, the world's entity pipelines). On a driver that
//! compiles slowly (a cold shader cache, llvmpipe, a weak CPU) that was seconds
//! of the main thread before the title showed.
//!
//! An object is kept under its descriptor's `Debug` text: wgpu prints the
//! labels, sources, constants, formats and states, and the handles a descriptor
//! names (shader modules, layouts) by their ids, so a descriptor that names a
//! kept handle matches only the same handle. Kept objects stay alive, so their
//! ids are never reused while they are keys.
use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};

/// One map of kept objects with the generation each was last asked for in.
struct Kept<T> {
    map: Mutex<HashMap<String, (T, u32)>>,
}
impl<T> Default for Kept<T> {
    fn default() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }
}
impl<T: Clone> Kept<T> {
    /// The object kept under `key`, else `make`'s, kept. The lock is not
    /// held while `make` compiles.
    fn get(&self, key: String, generation: u32, make: impl FnOnce() -> T) -> T {
        if let Some((object, used)) = self
            .map
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(&key)
        {
            *used = generation;
            return object.clone();
        }
        let object = make();
        self.map
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, (object.clone(), generation));
        object
    }
    fn keep_only(&self, generation: u32) {
        self.map
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|_, (_, used)| *used == generation);
    }
    fn len(&self) -> usize {
        self.map
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

/// The shader modules, layouts and pipelines of the device's renderers.
#[derive(Default)]
pub struct Reuse {
    generation: std::sync::atomic::AtomicU32,
    shaders: Kept<wgpu::ShaderModule>,
    bind_group_layouts: Kept<wgpu::BindGroupLayout>,
    pipeline_layouts: Kept<wgpu::PipelineLayout>,
    render: Kept<wgpu::RenderPipeline>,
    /// The menu blur's, for the swapchain format it was built for.
    pub menu_blur: Later<wgpu::TextureFormat, crate::menu_blur::MenuBlurPipeline>,
}

/// An object no frame needs until later (the menu blur, at the first blurred
/// screen), built on a worker thread when a renderer is, so the frame that
/// first needs it doesn't wait on its compile; vanilla compiles the blur with
/// the resources too. Kept for the device's later renderers, under the key
/// (the format) it was built for.
pub struct Later<K, T> {
    slot: Mutex<Option<(K, Building<T>)>>,
}
enum Building<T> {
    Started(std::thread::JoinHandle<T>),
    Done(T),
}
impl<K, T> Default for Later<K, T> {
    fn default() -> Self {
        Self {
            slot: Mutex::new(None),
        }
    }
}
impl<K: PartialEq, T: Clone + Send + 'static> Later<K, T> {
    /// Start building `key`'s on a worker thread, unless it is built or
    /// being built.
    pub fn start(&self, name: &str, key: K, make: impl FnOnce() -> T + Send + 'static) {
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.as_ref().is_some_and(|(k, _)| *k == key) {
            return;
        }
        *slot = std::thread::Builder::new()
            .name(name.into())
            .spawn(make)
            .ok()
            .map(|thread| (key, Building::Started(thread)));
    }
    /// `key`'s object: the worker's (waited for if it is still compiling),
    /// else `make`'s, kept.
    pub fn get(&self, key: K, make: impl FnOnce() -> T) -> T {
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        let built = match slot.take() {
            Some((k, Building::Started(thread))) if k == key => thread.join().ok(),
            Some((k, Building::Done(object))) if k == key => Some(object),
            _ => None,
        };
        let object = built.unwrap_or_else(make);
        *slot = Some((key, Building::Done(object.clone())));
        object
    }
}

impl Reuse {
    /// A renderer starts building: what it asks for is kept, the rest is let
    /// go by [`Reuse::finish`].
    pub fn begin(&self) -> u32 {
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .wrapping_add(1)
    }
    /// The renderer of `generation` is built: objects only older renderers
    /// used are dropped here (the renderers still alive hold their own).
    pub fn finish(&self, generation: u32) {
        self.shaders.keep_only(generation);
        self.bind_group_layouts.keep_only(generation);
        self.pipeline_layouts.keep_only(generation);
        self.render.keep_only(generation);
    }
    /// How many objects are kept.
    pub fn len(&self) -> usize {
        self.shaders.len()
            + self.bind_group_layouts.len()
            + self.pipeline_layouts.len()
            + self.render.len()
    }
    fn current(&self) -> u32 {
        self.generation.load(std::sync::atomic::Ordering::Relaxed)
    }
    /// `device`'s `create_*` calls through this cache.
    pub fn on<'a>(&'a self, device: &'a wgpu::Device) -> On<'a> {
        On {
            reuse: self,
            device,
            generation: self.current(),
        }
    }
}

/// A device whose shader modules, layouts and render pipelines come from a
/// [`Reuse`] when one with the same descriptor is kept.
pub struct On<'a> {
    reuse: &'a Reuse,
    device: &'a wgpu::Device,
    generation: u32,
}
impl On<'_> {
    pub fn create_shader_module(
        &self,
        descriptor: wgpu::ShaderModuleDescriptor<'_>,
    ) -> wgpu::ShaderModule {
        let key = format!("{descriptor:?}");
        self.reuse.shaders.get(key, self.generation, || {
            self.device.create_shader_module(descriptor)
        })
    }
    pub fn create_bind_group_layout(
        &self,
        descriptor: &wgpu::BindGroupLayoutDescriptor<'_>,
    ) -> wgpu::BindGroupLayout {
        let key = format!("{descriptor:?}");
        self.reuse.bind_group_layouts.get(key, self.generation, || {
            self.device.create_bind_group_layout(descriptor)
        })
    }
    pub fn create_pipeline_layout(
        &self,
        descriptor: &wgpu::PipelineLayoutDescriptor<'_>,
    ) -> wgpu::PipelineLayout {
        let key = format!("{descriptor:?}");
        self.reuse.pipeline_layouts.get(key, self.generation, || {
            self.device.create_pipeline_layout(descriptor)
        })
    }
    pub fn create_render_pipeline(
        &self,
        descriptor: &wgpu::RenderPipelineDescriptor<'_>,
    ) -> wgpu::RenderPipeline {
        let key = format!("{descriptor:?}");
        self.reuse.render.get(key, self.generation, || {
            self.device.create_render_pipeline(descriptor)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Kept, Later};

    /// What the worker built is what the first use gets, without building it
    /// again; a use under another key builds its own, which is kept.
    #[test]
    fn a_later_object_is_the_workers_for_its_key() {
        let later = Later::<u8, u32>::default();
        let (go, wait) = std::sync::mpsc::channel::<()>();
        later.start("test", 1, move || {
            // Still compiling when the first frame asks: it waits.
            wait.recv().unwrap();
            10
        });
        // Started again for the same key: nothing new is built.
        later.start("test", 1, || unreachable!("built twice"));
        go.send(()).unwrap();
        assert_eq!(later.get(1, || unreachable!("built on the frame")), 10);
        assert_eq!(later.get(1, || unreachable!("built on the frame")), 10);
        // The swapchain's format changed (HDR): built now, and kept.
        assert_eq!(later.get(2, || 20), 20);
        assert_eq!(later.get(2, || unreachable!("built twice")), 20);
        // Never started: built at the first use.
        assert_eq!(Later::<u8, u32>::default().get(1, || 30), 30);
    }

    /// A key asked for again gets the kept object without making one; what
    /// the latest generation did not ask for is let go when it finishes.
    #[test]
    fn kept_objects_are_shared_until_a_build_stops_asking() {
        let kept = Kept::<u32>::default();
        let made = std::cell::Cell::new(0);
        let make = |value| {
            made.set(made.get() + 1);
            value
        };
        assert_eq!(kept.get("a".into(), 1, || make(10)), 10);
        assert_eq!(kept.get("b".into(), 1, || make(20)), 20);
        kept.keep_only(1);
        assert_eq!(kept.len(), 2);
        // The next build asks for "a" only.
        assert_eq!(kept.get("a".into(), 2, || make(11)), 10);
        kept.keep_only(2);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept.get("b".into(), 3, || make(21)), 21);
        assert_eq!(made.get(), 3);
    }
}
