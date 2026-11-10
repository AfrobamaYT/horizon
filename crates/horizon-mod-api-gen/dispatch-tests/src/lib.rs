// Dispatch fixtures isolate listener ordering; API tests cover property validation.
#[cfg(test)]
mod item {
    pub(crate) mod crafting {
        pub(crate) fn validate_recipe_properties<T>(_: &str, _: &[T], _: &mut Vec<T>) {}
    }
}

#[cfg(test)]
mod tests {
    #[allow(dead_code)]
    mod stubs {
        include!(concat!(env!("OUT_DIR"), "/dispatch_fixtures_stubs.rs"));
    }
    use generated as mod_api;
    include!(concat!(env!("OUT_DIR"), "/dispatch_fixtures_root.rs"));
}
