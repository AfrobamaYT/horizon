mod api {
    pub struct ModMismatch;
}
mod crate_root {
    mod game {
        pub(crate) mod mismatch_resolution {
            pub(crate) fn attribute<T>(_: &str, _: &[T], _: &mut Vec<T>) {}
        }
    }
}
