use mod_points::endpoint_policy::{
    EndpointPolicy, InternalDeclaration, InternalField, PointSpec, SchemaCatalog,
};
use mod_points::exposure::{Field, Layout};
use std::sync::Arc;

fn layout(name: &str, fields: Vec<Field>, opaque: bool) -> Layout {
    Layout {
        name: name.into(),
        signature: name.into(),
        fields,
        children: vec![],
        opaque: opaque.then(|| "opaque".into()),
    }
}
fn field(name: &str, ordinal: u32, target: usize) -> Field {
    Field {
        case: None,
        ordinal,
        name: name.into(),
        target,
    }
}
fn catalog() -> SchemaCatalog {
    SchemaCatalog {
        layouts: vec![
            layout(
                "receiver",
                vec![field("visible", 0, 1), field("secret", 1, 2)],
                false,
            ),
            layout("scalar", vec![], false),
            layout("opaque", vec![], true),
            layout("call", vec![field("root0", 0, 0)], false),
        ],
        points: vec![PointSpec {
            head_call: 3,
            tail_call: None,
            receiver: Some(0),
        }],
        fields: vec![
            InternalField {
                path: "Game::visible".into(),
                kind: "field".into(),
                hash: "abc".into(),
                receiver: 0,
                field: "visible".into(),
            },
            InternalField {
                path: "Game::secret".into(),
                kind: "field".into(),
                hash: "def".into(),
                receiver: 0,
                field: "secret".into(),
            },
        ],
    }
}
fn declaration(path: &str, hash: &str) -> InternalDeclaration {
    InternalDeclaration {
        path: path.into(),
        kind: Some("field".into()),
        hash: hash.into(),
    }
}
#[test]
fn grants_are_per_module_validated_and_freed() {
    let c = catalog();
    let empty = EndpointPolicy::build(&c, &[]).unwrap();
    let granted = EndpointPolicy::build(&c, &[declaration("Game::visible", "abc")]).unwrap();
    let a = empty.plan(0).unwrap();
    let b = granted.plan(0).unwrap();
    assert!(!a.field(0, None, 0));
    assert!(b.field(0, None, 0));
    assert!(!b.field(0, None, 1));
    assert_ne!(a.key(0), b.key(0));
    assert!(EndpointPolicy::build(&c, &[declaration("Game::secret", "def")]).is_err());
    assert!(EndpointPolicy::build(&c, &[declaration("Game::missing", "abc")]).is_err());
    let weak = Arc::downgrade(&b);
    drop(b);
    drop(granted);
    assert!(weak.upgrade().is_none());
}
