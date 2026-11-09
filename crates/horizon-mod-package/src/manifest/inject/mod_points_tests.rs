use super::super::{Access, Kind};
use super::*;

fn catalog() -> Catalog {
    let point = |name: &str, target: &str, refusal: Option<&str>| Point {
        name: name.into(),
        target: target.into(),
        hash: format!("{name:0>16}").chars().take(16).collect(),
        signature: format!("{target}: fn f()"),
        raw: true,
        at_return: true,
        api_refusal: refusal.map(str::to_owned),
        returns: "cancel".into(),
        host: "none".into(),
        return_ty: "()".into(),
        listener: Some(format!("crate::__{name}Listener")),
        return_listener: Some(format!("crate::__{name}ReturnListener")),
    };
    Catalog {
        krate: "chemlib".into(),
        table: "__horizon_points".into(),
        opted_out: vec!["chemlib::secret".into()],
        points: vec![
            point("transfer", "chemlib::transfer", None),
            point(
                "opaque",
                "chemlib::opaque",
                Some("argument 1 `value` (`Opaque`) has no API type yet"),
            ),
        ],
    }
}

fn inject(at: &str, kind: Kind, access: Access, hash: &str) -> Inject {
    Inject {
        id: "listen".into(),
        at: at.into(),
        kind,
        with: "listener".into(),
        access,
        priority: 0,
        hash: hash.into(),
        call: None,
        nth: None,
        ctx: None,
    }
}

fn library(catalog: &Catalog, same_target: bool) -> Library<'_> {
    Library {
        id: "chemlib",
        version: "1.2.0",
        same_target,
        catalog,
    }
}

#[test]
fn an_injection_routes_to_its_place_by_access_and_kind() {
    let catalog = catalog();
    let same = library(&catalog, true);
    let hash = &catalog.points[0].hash;
    for (kind, access, place) in [
        (Kind::Head, Access::Api, "transfer"),
        (Kind::Replace, Access::Api, "transfer"),
        (Kind::Return, Access::Api, "transfer_return"),
        (Kind::Head, Access::Internals, "transfer"),
        (Kind::Return, Access::Internals, "transfer_return"),
    ] {
        let routed = route(
            "addon",
            &inject("chemlib::transfer", kind, access, hash),
            &same,
        )
        .unwrap();
        assert_eq!((routed.place.as_str(), routed.access), (place, access));
    }
    assert!(catalog.owns("chemlib::tanks::fill"));
    assert!(!catalog.owns("chemlibx::fill"));
}

#[test]
fn each_refusal_names_what_changed_or_why() {
    let catalog = catalog();
    let same = library(&catalog, true);
    let other = library(&catalog, false);
    let hash = catalog.points[0].hash.clone();
    let opaque = catalog.points[1].hash.clone();
    let refused = |inject: Inject, library: &Library<'_>| {
        route("addon", &inject, library).unwrap_err().to_string()
    };
    assert_eq!(
        refused(
            inject(
                "chemlib::transfer",
                Kind::Head,
                Access::Api,
                "0000000000000000"
            ),
            &same
        ),
        format!(
            "E_INJECT_CHANGED: addon:listen: chemlib 1.2.0 changed chemlib::transfer: pinned \
             0000000000000000, installed {hash} (chemlib::transfer: fn f()); update the \
             declaration with `cargo horizon-mod inject update` and rebuild against it"
        )
    );
    assert_eq!(
        refused(
            inject("chemlib::secret", Kind::Head, Access::Api, &hash),
            &same
        ),
        "E_INJECT_MISSING: addon:listen: chemlib::secret has no point (opted out with #[mod_api::no_point])"
    );
    assert!(
        refused(
            inject("chemlib::gone", Kind::Head, Access::Api, &hash),
            &same
        )
        .starts_with("E_INJECT_MISSING")
    );
    assert!(
        refused(
            inject("chemlib::opaque", Kind::Head, Access::Api, &opaque),
            &same
        )
        .contains("argument 1 `value` (`Opaque`) has no API type yet")
    );
    // Internals reach the library's own types: refused across targets, whatever the record.
    assert_eq!(
        refused(
            inject("chemlib::transfer", Kind::Head, Access::Internals, &hash),
            &other
        ),
        "addon:listen: internals access to chemlib needs the same target: the same binary on \
         native, the same dependency group on wasm"
    );
    assert_eq!(
        refused(
            inject("chemlib::opaque", Kind::Head, Access::Api, &opaque),
            &other
        ),
        "addon:listen: an api listener at chemlib on the other target crosses through the \
         game, and argument 1 `value` (`Opaque`) has no API type yet: add \
         #[derive(mod_api::Wire)] to it"
    );
    assert_eq!(
        refused(
            inject("chemlib::transfer", Kind::Head, Access::Api, &hash),
            &other
        ),
        "addon:listen: chemlib is on the other target: mixed native and wasm dependencies \
         aren't supported yet"
    );
    // The opaque point keeps its internals form on the same target.
    assert!(
        route(
            "addon",
            &inject("chemlib::opaque", Kind::Head, Access::Internals, &opaque),
            &same
        )
        .is_ok()
    );
    let mut call = inject("chemlib::transfer", Kind::BeforeCall, Access::Api, &hash);
    call.call = Some("chemlib::helper".into());
    assert!(refused(call, &same).starts_with("E_INJECT_NO_SLOT"));
}
