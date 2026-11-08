fn main() {
    let root =
        std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    horizon_mod_api_gen::generate_points(&root, &out_dir).expect("generating the points");
    let rule = horizon_mod_api_gen::crossing::Rule::new(&[]);
    for ty in [
        "Vec<u32>",
        "Option<String>",
        "(u32, String)",
        "std::collections::BTreeMap<u32, String>",
    ] {
        let generated = horizon_mod_api_gen::native_window::generate(
            &rule,
            "horizon_client",
            None,
            &syn::parse_str(ty).unwrap(),
            quote::quote!(value),
            &horizon_mod_api_gen::native_schema::All,
        )
        .unwrap();
        let name = match ty {
            "Vec<u32>" => "vec",
            "Option<String>" => "option",
            "(u32, String)" => "tuple",
            _ => "map",
        };
        std::fs::write(
            out_dir.join(format!("native_window_{name}.rs")),
            generated.to_string(),
        )
        .unwrap();
    }
}
