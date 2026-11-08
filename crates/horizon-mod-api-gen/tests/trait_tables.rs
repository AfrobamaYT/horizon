//! The real catalog's traits as method tables: which methods cross as callables, each gap with
//! its reason. `TRAIT_TABLE=path` writes the full per-method table.
use horizon_mod_api_gen::{
    crossing::Rule,
    native_callable::{method_gap, methods},
};

#[test]
fn catalog_traits_cross_per_method() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let entries = horizon_mod_api_gen::internals::rule_catalog(&root).unwrap();
    let rule = Rule::new(&entries);
    let mut table = String::new();
    let mut crossing = std::collections::BTreeMap::new();
    for entry in entries.iter().filter(|entry| entry.kind == "trait") {
        let methods = methods(&rule, &entry.path);
        let mut count = 0;
        table.push_str(&format!("TRAIT {}\n", entry.path));
        for method in &methods {
            let gap = method_gap(&rule, &entry.path, &method.name.to_string());
            count += usize::from(gap.is_none());
            table.push_str(&format!(
                "  {}{} {}\n",
                method.name,
                if method.exclusive { " (&mut self)" } else { "" },
                gap.as_deref().unwrap_or("crosses")
            ));
        }
        crossing.insert(entry.path.clone(), (count, methods.len()));
    }
    if let Ok(path) = std::env::var("TRAIT_TABLE") {
        std::fs::write(path, table).unwrap();
    }
    // The 57+1 `&dyn EventLoopCtl` points: both methods cross.
    assert_eq!(
        crossing["horizon_client::game::event_loop::EventLoopCtl"],
        (2, 2)
    );
    assert_eq!(
        crossing["horizon_client_input::clipboard::Clipboard"],
        (2, 2)
    );
}
