use super::*;
use crate::crossing::tests::internal;

fn catalog(parameter: &str) -> Vec<Internal> {
    vec![
        internal(
            "horizon_client::game::Game",
            "struct",
            "struct Game { child: Child }",
        ),
        internal("horizon_client::game::Game::child", "field", "Child"),
        internal("horizon_client::game::Child", "struct", "struct Child {}"),
        internal(
            "horizon_client::game::Game::tick",
            "method",
            "impl Game { fn tick(&mut self) }",
        ),
        internal(
            "horizon_client::game::Child::step",
            "method",
            &format!("impl Child {{ fn step(&mut self, value: {parameter}) -> u32 }}"),
        ),
        internal(
            "horizon_client::game::add",
            "fn",
            "fn add(value: u32) -> u32",
        ),
    ]
}

#[test]
fn site_lists_are_a_sorted_union_with_locations_and_checked_syntax() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("site-lists-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let a = directory.join("a.txt");
    let b = directory.join("b.txt");
    fs::write(
        &a,
        "# comment\ngame::Game<'_>::tick | self.step | 0 # note\ngame::tick | .step | \n",
    )
    .unwrap();
    fs::write(&b, "game::Game::tick | self.step | 0\n").unwrap();
    let declarations = listed(&directory).unwrap();
    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].target, "game::Game::tick");
    assert!(declarations[0].origin.ends_with("a.txt:2"));
    assert_eq!(declarations[1].ordinal, None);
    for (bad, reason) in [
        ("game::tick | self.step", "expected target"),
        ("? | self.step | 0", "target:"),
        ("game::tick | . | 0", "selector"),
        ("game::tick | .step | -1", "ordinal"),
    ] {
        fs::write(&a, bad).unwrap();
        let error = listed(&directory).unwrap_err();
        assert!(
            error.contains("a.txt:1") && error.contains(reason),
            "{error}"
        );
    }
    fs::remove_dir_all(&directory).unwrap();
    assert!(listed(&directory).unwrap().is_empty());
}

#[test]
fn site_resolution_walks_fields_and_annotated_parameters_and_path_calls() {
    let entries = catalog("u32");
    let rule = Rule::new(&entries);
    let sig: syn::Signature = syn::parse_quote!(fn tick(&mut self, child: &mut Child));
    for call in [
        "self.child.step(7)",
        "child.step(7)",
        "add(7)",
        "horizon_client::game::add(7)",
    ] {
        let expr = syn::parse_str(call).unwrap();
        let resolved = resolve(&expr, &entries[3], &sig, &entries, &rule).unwrap();
        assert_eq!(resolved.fields[0], "value");
        assert_eq!(resolved.types[0].to_token_stream().to_string(), "u32");
        assert_eq!(resolved.output.to_string(), "u32");
    }
}

#[test]
fn site_resolution_refuses_incompatible_callee_signatures() {
    for (signature, reason) in [
        (
            "impl<T> Child { fn step(&mut self, value: u32) }",
            "generic or bounded",
        ),
        (
            "impl Child { fn step<T>(&mut self, value: u32) }",
            "generic, unsafe",
        ),
        (
            "impl Child { unsafe fn step(&mut self, value: u32) }",
            "generic, unsafe",
        ),
        (
            "impl Child { fn step(self, value: u32) }",
            "by-value receiver",
        ),
        (
            "impl Child { fn step(&mut self, (value,): (u32,)) }",
            "destructured",
        ),
        (
            "impl Child { fn step(&mut self, value: u32) -> &u32 }",
            "borrowed or unresolved",
        ),
    ] {
        let mut entries = catalog("u32");
        entries[4].signature = signature.into();
        let rule = Rule::new(&entries);
        let expr = syn::parse_quote!(self.child.step(1));
        let sig = syn::parse_quote!(fn tick(&mut self));
        let error = resolve(&expr, &entries[3], &sig, &entries, &rule)
            .err()
            .unwrap();
        assert!(error.contains(reason), "{signature}: {error}");
    }
}

#[test]
fn site_resolution_refuses_unresolved_receivers_and_non_record_shapes() {
    for (parameter, call, reason) in [
        (
            "u32",
            "local.step(1)",
            "local has no parameter type annotation",
        ),
        ("u32", "self.missing.step(1)", "unresolved receiver field"),
        ("u32", "self.make().step(1)", "chained calls"),
        ("u32", "self.child.step::<u8>(1)", "generic callee"),
        ("T", "self.child.step(1)", "borrowing or non-Copy"),
        ("&u32", "self.child.step(1)", "borrowing or non-Copy"),
        ("String", "self.child.step(1)", "borrowing or non-Copy"),
        ("u32", "self.child.step(self.x)", "two-phase borrowing"),
        ("u32", "self.child.step()", "argument count"),
        ("u32", "unknown(1)", "callee not in"),
    ] {
        let entries = catalog(parameter);
        let rule = Rule::new(&entries);
        let sig = syn::parse_quote!(fn tick(&mut self));
        let expr = syn::parse_str(call).unwrap();
        let error = resolve(&expr, &entries[3], &sig, &entries, &rule)
            .err()
            .unwrap();
        assert!(error.contains(reason), "{call}: {error}");
    }
}
