fn main() {
    let root =
        std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    horizon_mod_api_gen::generate_points(&root, &out_dir).expect("generating the points");
    receiver_fixture(&out_dir);
    // Exercise the actual catalog signature and Rust body, not a handwritten nominal DTO.
    let source = root.join("crates/horizon-client/src/game/network/server_links.rs");
    println!("cargo:rerun-if-changed={}", source.display());
    let parsed = syn::parse_file(&std::fs::read_to_string(source).unwrap()).unwrap();
    let mut function = parsed
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == "link_buttons" => {
                Some(function.clone())
            }
            _ => None,
        })
        .unwrap();
    let attribute = quote::quote!(name = "game_server_links_link_buttons", raw, free);
    let shape = point_shape::shape(attribute.clone(), &function.sig).unwrap();
    let rule = horizon_mod_api_gen::crossing::Rule::new(&[]);
    let plan = horizon_mod_api_gen::native_raw::generate(
        &rule,
        "horizon_client::game::server_links",
        &function.sig,
        &shape,
    )
    .unwrap()
    .attribute();
    let raw_types = shape.raw_items(&function.sig, None, None);
    function.attrs.clear();
    let constants = parsed.items.iter().filter(|item| matches!(item, syn::Item::Const(item) if ["LINK_WIDTH", "ROW", "SCROLL_RESERVE"].iter().any(|name| item.ident == *name)));
    std::fs::write(
        out_dir.join("native_raw_link_buttons.rs"),
        quote::quote! {
            #(#constants)*
            #raw_types
            #[horizon_mod_api_macros::point(#attribute)]
            #plan
            #function
        }
        .to_string(),
    )
    .unwrap();
    let fixture = horizon_mod_api_gen::internals::Internal {
        path: "horizon_client::Choice".into(),
        kind: "enum",
        cfg: String::new(),
        signature: "pub enum Choice { Empty, One(u32), Named { value: String } }".into(),
        hash: String::new(),
        shape_only: false,
        opaque_reason: None,
        refs: Default::default(),
    };
    let pair = horizon_mod_api_gen::internals::Internal {
        path: "horizon_client::Pair".into(),
        kind: "struct",
        cfg: String::new(),
        signature: "pub struct Pair { pub a: u32, pub b: u32 }".into(),
        hash: String::new(),
        shape_only: false,
        opaque_reason: None,
        refs: Default::default(),
    };
    let later = horizon_mod_api_gen::internals::Internal {
        path: "horizon_client::Later".into(),
        kind: "enum",
        cfg: String::new(),
        signature: "pub enum Later { Both { a: u32, b: u32 } }".into(),
        hash: String::new(),
        shape_only: false,
        opaque_reason: None,
        refs: Default::default(),
    };
    let leaf = horizon_mod_api_gen::internals::Internal {
        path: "horizon_client::Leaf".into(),
        kind: "struct",
        cfg: String::new(),
        signature: "pub struct Leaf { pub n: u32 }".into(),
        hash: String::new(),
        shape_only: false,
        opaque_reason: None,
        refs: Default::default(),
    };
    let holder = horizon_mod_api_gen::internals::Internal {
        path: "horizon_client::Holder".into(),
        kind: "struct",
        cfg: String::new(),
        signature: "pub struct Holder { pub child: Box<Leaf>, pub items: &'static [u32], pub shared: &'static Leaf }".into(),
        hash: String::new(),
        shape_only: false,
        opaque_reason: None,
        refs: Default::default(),
    };
    let rule = horizon_mod_api_gen::crossing::Rule::new(&[fixture, pair, later, leaf, holder]);
    for ty in [
        "horizon_client::Pair",
        "horizon_client::Later",
        "horizon_client::Holder",
        "Box<Vec<u32>>",
        "&'static [u32]",
        "Vec<u32>",
        "Option<String>",
        "(u32, String)",
        "std::collections::BTreeMap<u32, String>",
        "horizon_client::Choice",
    ] {
        let exposure: &dyn horizon_mod_api_gen::native_schema::Exposure =
            if ty.ends_with("Pair") || ty.ends_with("Later") {
                &OnlyB
            } else {
                &horizon_mod_api_gen::native_schema::All
            };
        let generated = horizon_mod_api_gen::native_window::generate(
            &rule,
            "horizon_client",
            None,
            &syn::parse_str(ty).unwrap(),
            quote::quote!(value),
            exposure,
        )
        .unwrap();
        let name = match ty {
            "Vec<u32>" => "vec",
            "Option<String>" => "option",
            "(u32, String)" => "tuple",
            "horizon_client::Choice" => "choice",
            "horizon_client::Pair" => "pair",
            "horizon_client::Holder" => "holder",
            "Box<Vec<u32>>" => "boxed",
            "&'static [u32]" => "slice",
            "horizon_client::Later" => "later",
            _ => "map",
        };
        std::fs::write(
            out_dir.join(format!("native_window_{name}.rs")),
            generated.to_string(),
        )
        .unwrap();
    }
}

/// Exposes only the second declared field of the sparse fixtures.
struct OnlyB;
impl horizon_mod_api_gen::native_schema::Exposure for OnlyB {
    fn name(&self) -> &str {
        "only-b"
    }
    fn field(&self, _: &str, _: Option<u32>, field: u32) -> bool {
        field == 1
    }
}

/// A real catalog `&self` point on a stand-in receiver, generated through the whole-catalog
/// graph, registration policy and once-per-source helpers.
fn receiver_fixture(out_dir: &std::path::Path) {
    use horizon_mod_api_gen::{
        crossing::Rule, internals::Internal, native_patch, native_policy::Graph, native_raw,
        native_window,
    };
    let entry = Internal {
        path: "horizon_client::model::Loading".into(),
        kind: "struct",
        cfg: String::new(),
        signature: "pub struct Loading { pub hidden: u32, pub size: [u32; 2] }".into(),
        hash: String::new(),
        shape_only: false,
        opaque_reason: None,
        refs: Default::default(),
    };
    let rule = Rule::new(&[entry]);
    let module = "horizon_client::model";
    let owner: syn::Type = syn::parse_str("Loading").unwrap();
    let signature: syn::Signature = syn::parse_str("fn menu_size(&self) -> [u32; 2]").unwrap();
    let attribute = quote::quote!(name = "game_startup_menu_loading_menu_size", raw);
    let shape = point_shape::shape(attribute.clone(), &signature).unwrap();
    let mut graph = Graph::default();
    let call =
        native_raw::register(&rule, module, Some(&owner), &signature, &shape, &mut graph).unwrap();
    let plan = native_raw::generate_runtime(
        &rule,
        module,
        Some(&owner),
        &signature,
        &shape,
        &graph,
        call,
    )
    .unwrap()
    .attribute();
    let receiver = graph.id(&rule.shape(module, None, &owner)).unwrap();
    let layouts = graph.tokens();
    let windows = native_window::helpers(&rule, &graph).unwrap();
    let patches = native_patch::helpers(&rule, &graph).unwrap();
    let (head, tail) = (
        call.head,
        call.tail
            .map_or(quote::quote!(None), |t| quote::quote!(Some(#t))),
    );
    let raw_types = shape.raw_items(&signature, Some(&owner), None);
    std::fs::write(
        out_dir.join("native_raw_receiver.rs"),
        quote::quote! {
            pub mod __horizon_points { #windows #patches }
            fn catalog() -> mod_points::endpoint_policy::SchemaCatalog {
                use mod_points::endpoint_policy::{InternalField, PointSpec, SchemaCatalog};
                SchemaCatalog {
                    layouts: #layouts,
                    points: vec![PointSpec { head_call: #head, tail_call: #tail, receiver: Some(#receiver) }],
                    fields: ["hidden", "size"].map(|field| InternalField {
                        path: format!("horizon_client::model::Loading::{field}"),
                        kind: "field".into(),
                        hash: field.into(),
                        receiver: #receiver,
                        field: field.into(),
                    }).to_vec(),
                }
            }
            const HEAD_CALL: usize = #head;
            #raw_types
            impl model::Loading {
                #[horizon_mod_api_macros::point(#attribute)]
                #plan
                fn menu_size(&self) -> [u32; 2] { self.size }
            }
        }
        .to_string(),
    )
    .unwrap();
}
