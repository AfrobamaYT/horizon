//! A wasm mod's declared game internals, generated at build time from its manifest and the
//! target game's catalog. Add this crate only when using `[native] internals` in a wasm mod.
//! `cargo horizon-mod build --wasm` supplies the manifest and catalog to the generator.
include!(concat!(env!("OUT_DIR"), "/view.rs"));

/// Generic call-frame machinery. API-specific wrappers and codecs are all generated.
#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub mod runtime {
    pub use bindings::exports::horizon::mod_internals::raw_listeners::{Guest, HostKind};
    pub use bindings::horizon::mod_internals::view::{Arg, Node, Reached, Refusal, call, root};
    pub use mod_points::internals::bindings;
    pub use mod_points::internals::{Value, flatten, unflatten};
    use std::marker::PhantomData;

    pub fn changed(message: &str) -> Refusal {
        Refusal::Changed(message.into())
    }
    pub fn wire_refusal(why: mod_points::internals::Refusal) -> Refusal {
        match why {
            mod_points::internals::Refusal::Gone(s) => Refusal::Gone(s),
            mod_points::internals::Refusal::Changed(s) => Refusal::Changed(s),
            mod_points::internals::Refusal::NotHere(s) => Refusal::NotHere(s),
        }
    }
    pub trait CopyValue: Sized {
        fn to_value(&self) -> Value;
        fn from_value(value: Value) -> Result<Self, Refusal>;
    }
    pub trait ViewType: Sized {
        fn from_reached(reached: Reached) -> Result<Self, Refusal>;
    }
    pub trait AsNode {
        fn node(&self) -> &Node;
    }

    /// A copy-typed root passed by mutable reference to a declared function.
    pub struct CopyNode<T>(Node, PhantomData<T>);
    impl<T: CopyValue> CopyNode<T> {
        pub fn read(&self) -> Result<T, Refusal> {
            T::from_value(unflatten(self.0.read()?).map_err(wire_refusal)?)
        }
    }
    impl<T> AsNode for CopyNode<T> {
        fn node(&self) -> &Node {
            &self.0
        }
    }
    impl<T> ViewType for CopyNode<T> {
        fn from_reached(reached: Reached) -> Result<Self, Refusal> {
            match reached {
                Reached::Node(n) => Ok(Self(n, PhantomData)),
                _ => Err(changed("a copy node")),
            }
        }
    }

    /// A sequence node; elements are copied or reached as call-scoped nodes by their generated type.
    pub struct Seq<T>(Node, PhantomData<T>);
    impl<T: ViewType> Seq<T> {
        pub fn len(&self) -> Result<u64, Refusal> {
            self.0.len()
        }
        pub fn is_empty(&self) -> Result<bool, Refusal> {
            Ok(self.len()? == 0)
        }
        pub fn get(&self, index: u64) -> Result<T, Refusal> {
            T::from_reached(self.0.index(index)?)
        }
    }
    impl<T> AsNode for Seq<T> {
        fn node(&self) -> &Node {
            &self.0
        }
    }
    impl<T> ViewType for Seq<T> {
        fn from_reached(reached: Reached) -> Result<Self, Refusal> {
            match reached {
                Reached::Node(n) => Ok(Self(n, PhantomData)),
                _ => Err(changed("a sequence node")),
            }
        }
    }
    /// A map node. Keys use the same generated copy codec as arguments and fields.
    pub struct Map<K, V>(Node, PhantomData<(K, V)>);
    impl<K: CopyValue, V: ViewType> Map<K, V> {
        pub fn len(&self) -> Result<u64, Refusal> {
            self.0.len()
        }
        pub fn is_empty(&self) -> Result<bool, Refusal> {
            Ok(self.len()? == 0)
        }
        pub fn get(&self, key: &K) -> Result<V, Refusal> {
            V::from_reached(self.0.key(&flatten(key.to_value()))?)
        }
    }
    impl<K, V> AsNode for Map<K, V> {
        fn node(&self) -> &Node {
            &self.0
        }
    }
    impl<K, V> ViewType for Map<K, V> {
        fn from_reached(reached: Reached) -> Result<Self, Refusal> {
            match reached {
                Reached::Node(n) => Ok(Self(n, PhantomData)),
                _ => Err(changed("a map node")),
            }
        }
    }
    /// A handle-typed option. A missing value is not a stale node.
    pub struct Opt<T>(Node, PhantomData<T>);
    impl<T: ViewType> Opt<T> {
        pub fn case(&self) -> Result<u32, Refusal> {
            self.0.case()
        }
        pub fn some(&self) -> Result<Option<T>, Refusal> {
            match self.case()? {
                0 => Ok(None),
                1 => Ok(Some(T::from_reached(Reached::Node(self.0.field(0)?))?)),
                _ => Err(changed("an option case")),
            }
        }
    }
    impl<T> AsNode for Opt<T> {
        fn node(&self) -> &Node {
            &self.0
        }
    }
    impl<T> ViewType for Opt<T> {
        fn from_reached(reached: Reached) -> Result<Self, Refusal> {
            match reached {
                Reached::Node(n) => Ok(Self(n, PhantomData)),
                _ => Err(changed("an option node")),
            }
        }
    }
}
