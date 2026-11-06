use super::*;

#[test]
fn lexical_bindings_and_macro_tokens_cannot_be_captured_as_generated_names() {
    let sig = syn::parse_quote!(fn run(add: fn(u32) -> u32, child: &Child));
    let body = syn::parse_quote!({
        let (local, mut nested) = pair();
        let closure = |argument| match argument {
            Some(matched) => matched,
            None => 0,
        };
        let __recv = 0;
        let __recv_1 = 0;
        opaque!(__recv_2);
        use other::callee as imported;
        const INNER: u32 = 1;
        fn inner_fn(unrelated: u32) {
            let nested_body = unrelated;
        }
    });
    let mut scope = Scope::scan(&sig, Some(&body));
    assert_eq!(
        scope.parameters,
        BTreeSet::from(["add".into(), "child".into()])
    );
    for name in [
        "local", "nested", "closure", "argument", "matched", "imported", "INNER", "inner_fn",
    ] {
        assert!(scope.locals.contains(name), "{name}");
    }
    assert!(!scope.locals.contains("unrelated"));
    assert!(!scope.locals.contains("nested_body"));
    assert!(scope.opaque);
    assert_eq!(scope.fresh("__recv"), "__recv_3");
    assert_eq!(scope.fresh("__recv"), "__recv_4");
    let glob = syn::parse_quote!({
        use other::*;
    });
    assert!(Scope::scan(&sig, Some(&glob)).opaque);
}

#[test]
fn runtime_call_selection_does_not_visit_any_constant_context() {
    let body = syn::parse_quote!({
        let _: [u8; add(10)] = [];
        let _ = [add(1); add(11)];
        let _ = holder::<{ add(12) }>(add(2));
        const C: usize = add(13);
        static S: usize = add(14);
        enum E {
            Value = add(15) as isize,
        }
        let _ = const { add(16) };
        type Array = [u8; add(17)];
        let _ = |x| add(x);
    });
    let calls = calls(&body, "add")
        .into_iter()
        .map(|e| e.to_token_stream().to_string().replace(' ', ""))
        .collect::<Vec<_>>();
    assert_eq!(calls, ["add(1)", "add(2)", "add(x)"]);
}

#[test]
fn resolved_shapes_find_elided_and_nested_lifetimes_before_spelling_results() {
    use crate::crossing::{Rule, tests::internal};
    let entries = [
        internal(
            "horizon_client::game::View",
            "struct",
            "struct View<'a>(&'a u32);",
        ),
        internal(
            "horizon_client::game::Owned",
            "struct",
            "struct Owned(u32);",
        ),
        internal(
            "horizon_client::game::Alias",
            "type",
            "type Alias<'a> = View<'a>;",
        ),
    ];
    let rule = Rule::new(&entries);
    for written in [
        "View",
        "View<'_>",
        "View<'static>",
        "Option<View>",
        "Vec<View>",
        "[View; 3]",
        "(u32, View)",
        "Alias",
        "std::borrow::Cow<str>",
        "&'static u32",
        "impl Iterator<Item = u32>",
    ] {
        let ty = syn::parse_str(written).unwrap();
        assert!(
            rule.shape("horizon_client::game", None, &ty).may_borrow(),
            "{written}"
        );
    }
    for written in [
        "Owned",
        "Option<Owned>",
        "Vec<u32>",
        "String",
        "()",
        "[u32; 3]",
        "(u32, bool)",
    ] {
        let ty = syn::parse_str(written).unwrap();
        assert!(
            !rule.shape("horizon_client::game", None, &ty).may_borrow(),
            "{written}"
        );
    }
}
