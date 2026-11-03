use super::inject;

fn injection(name: &str, event: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "event": event,
        "declaration": format!("fn {name}(&self) {{}}"),
        "event_declaration": format!("pub struct {event};"),
    })
}

#[test]
fn injected_methods_get_matching_ids_and_counts() {
    let api = "pub enum HookId { Tick } pub(crate) const HOOK_COUNT: usize = 1; pub mod hook_ids { pub const tick: super::HookId = super::HookId::Tick; } pub trait Hooks {}";
    let output = inject(
        api,
        &serde_json::json!([injection("injected", "InjectedEvent")]).to_string(),
    )
    .unwrap();
    let file = syn::parse_file(&output).unwrap();
    let ids = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Enum(item) if item.ident == "HookId" => Some(item),
            _ => None,
        })
        .unwrap();
    assert_eq!(ids.variants.len(), 2);
    assert_eq!(ids.variants[1].ident, "Injected");
    let constants = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Mod(item) if item.ident == "hook_ids" => item.content.as_ref(),
            _ => None,
        })
        .unwrap();
    assert!(
        constants
            .1
            .iter()
            .any(|item| matches!(item, syn::Item::Const(item) if item.ident == "injected"))
    );
    assert!(output.contains("HOOK_COUNT : usize = 2"));
}

#[test]
fn rejects_duplicate_methods_and_event_names_across_injections() {
    let api = "pub trait Hooks: Send + Sync {}";
    for declarations in [
        vec![
            injection("tick", "TickEvent"),
            injection("tick", "OtherEvent"),
        ],
        vec![
            injection("tick", "TickEvent"),
            injection("other", "TickEvent"),
        ],
    ] {
        assert!(inject(api, &serde_json::to_string(&declarations).unwrap()).is_err());
    }
}

#[test]
fn rejects_malformed_or_misnamed_declarations_at_the_boundary() {
    let api = "pub trait Hooks: Send + Sync {}";
    let mut wrong_method = injection("tick", "TickEvent");
    wrong_method["declaration"] = "fn other(&self) {}".into();
    let mut wrong_event = injection("tick", "TickEvent");
    wrong_event["event_declaration"] = "pub struct OtherEvent;".into();
    let mut invalid_rust = injection("tick", "TickEvent");
    invalid_rust["event_declaration"] = "not a struct".into();
    for declaration in [wrong_method, wrong_event, invalid_rust] {
        assert!(inject(api, &serde_json::json!([declaration]).to_string()).is_err());
    }
    for invalid in ["{}", "[{}]", "null", "not json"] {
        assert!(inject(api, invalid).is_err());
    }
}
