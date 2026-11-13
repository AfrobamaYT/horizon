//! `text_from_json`: a text component read as `/tellraw` takes it.
use super::Game;
use horizon_client::component;
use horizon_client::text::Text;
use serde_json::{Map, Value};

impl Game<'_> {
    /// A component read as `/tellraw` takes it; None when it isn't one (an object with no
    /// content key, an empty list, a content key of the wrong kind).
    pub(super) fn text_from_json(&self, json: &str) -> Option<Text> {
        component_from_json(json)
    }
}

/// One JSON or SNBT component (`ComponentSerialization.CODEC`'s string, list or object
/// forms), with nothing but whitespace after it.
fn component_from_json(json: &str) -> Option<Text> {
    let (value, used) = horizon_client::component::parse_snbt(json).ok()?;
    if !json.as_bytes()[used..].iter().all(u8::is_ascii_whitespace) {
        return None;
    }
    if !is_component(&value) {
        return None;
    }
    Some(Text::from(value))
}

/// Whether `value` decodes under `ComponentSerialization.CODEC`: a string, a non-empty list of
/// components, or an object with one content (`text`, `translate`, `keybind`, `score`,
/// `selector`, `nbt` with its source, or an inline object) and well-formed `extra`. An object
/// naming a `type` is read as that type only (`StrictEither`); one without is read by the
/// content keys it has.
fn is_component(value: &Value) -> bool {
    match value {
        Value::String(_) => true,
        Value::Array(items) => !items.is_empty() && items.iter().all(is_component),
        Value::Object(fields) => {
            let content = match fields.get("type") {
                None => [
                    "text",
                    "translatable",
                    "keybind",
                    "score",
                    "selector",
                    "nbt",
                    "object",
                ]
                .into_iter()
                .any(|kind| has_content(fields, kind)),
                Some(Value::String(kind)) => has_content(fields, kind),
                Some(_) => false,
            };
            content
                && fields.get("extra").is_none_or(|extra| {
                    extra.as_array().is_some_and(|l| l.iter().all(is_component))
                })
        }
        _ => false,
    }
}

/// Whether `fields` hold the content `kind` (a `ComponentContents` type name) with its
/// required keys of the right kind.
fn has_content(fields: &Map<String, Value>, kind: &str) -> bool {
    let string = |key: &str| fields.get(key).is_some_and(Value::is_string);
    let optional_component = |key: &str| fields.get(key).is_none_or(is_component);
    match kind {
        "text" => fields
            .get("text")
            .is_some_and(|text| text.is_string() || text.is_number()),
        "translatable" => {
            string("translate")
                && fields.get("fallback").is_none_or(Value::is_string)
                && fields.get("with").is_none_or(|with| {
                    with.as_array().is_some_and(|args| {
                        args.iter()
                            .all(|arg| is_component(arg) || arg.is_number() || arg.is_boolean())
                    })
                })
        }
        "keybind" => string("keybind"),
        "selector" => string("selector") && optional_component("separator"),
        "score" => fields
            .get("score")
            .and_then(Value::as_object)
            .is_some_and(|score| {
                ["name", "objective"]
                    .iter()
                    .all(|key| score.get(*key).is_some_and(Value::is_string))
            }),
        "nbt" => {
            string("nbt")
                && optional_component("separator")
                && fields.get("interpret").is_none_or(Value::is_boolean)
                && match fields.get("source") {
                    None => ["entity", "block", "storage"].iter().any(|key| string(key)),
                    Some(Value::String(source)) => {
                        matches!(source.as_str(), "entity" | "block" | "storage") && string(source)
                    }
                    Some(_) => false,
                }
        }
        "object" => {
            component::inline_object_fields(fields).is_some()
                && fields.get("fallback").is_none_or(is_component)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{Text, component_from_json};
    use crate::game::api_values::TryIntoApi;
    use mod_api::text::{TextClick, TextContent};

    fn api(text: &Text) -> mod_api::Text {
        text.try_into_api().unwrap()
    }

    /// JSON and SNBT components keep their structure: styles, children, translations
    /// and click events; what is not one component is refused.
    #[test]
    fn text_from_json_reads_components_as_tellraw_does() {
        let text = api(&component_from_json(
            r#"{"text":"Hi ","color":"gold","extra":[{"text":"there","bold":true}]}"#,
        )
        .unwrap());
        assert_eq!(text.content, TextContent::Literal("Hi ".into()));
        assert_eq!(text.style.color.as_deref(), Some("gold"));
        assert_eq!(text.children.len(), 1);
        assert_eq!(text.children[0].style.bold, Some(true));
        let text = api(
            &component_from_json(r#"{"translate":"chat.type.text","with":["a","b"]}"#).unwrap(),
        );
        assert!(
            matches!(&text.content, TextContent::Translatable { key, arguments, .. } if key == "chat.type.text" && arguments.len() == 2)
        );
        let text = api(&component_from_json(
            "{text:'run',click_event:{action:run_command,command:'/spawn'}}",
        )
        .unwrap());
        assert_eq!(
            text.style.click,
            Some(TextClick::RunCommand("/spawn".into()))
        );
        assert_eq!(
            api(&component_from_json("  \"plain\"\n").unwrap()).content,
            TextContent::Literal("plain".into())
        );
        for refused in [
            "",
            "{\"text\":\"a\"} trailing",
            "{\"text\":\"open",
            "12",
            "true",
            "[]",
        ] {
            assert!(component_from_json(refused).is_none(), "{refused:?}");
        }
    }

    /// Objects and lists that are not one component are refused whole, however deep.
    const NOT_COMPONENTS: [&str; 24] = [
        "{}",
        r#"{"color":"red"}"#,
        r#"{"unrelated":"x","extra":[]}"#,
        r#"{"bold":true,"italic":false}"#,
        r#"{"text":{"a":1}}"#,
        r#"{"text":["a"]}"#,
        r#"{"translate":5}"#,
        r#"{"translate":"k","with":"a"}"#,
        r#"{"keybind":true}"#,
        r#"{"selector":["@a"]}"#,
        r#"{"score":{"name":"a"}}"#,
        r#"{"score":"a"}"#,
        r#"{"nbt":"a.b"}"#,
        r#"{"nbt":"a.b","source":"entity","block":"0 0 0"}"#,
        r#"{"type":"bogus","text":"a"}"#,
        r#"{"type":"keybind","text":"a"}"#,
        r#"{"text":"a","extra":"b"}"#,
        r#"{"text":"a","extra":[{}]}"#,
        r#"{"text":"a","extra":[{"text":"b","extra":[{"color":"red"}]}]}"#,
        r#"[{}]"#,
        r#"["a",{"color":"red"}]"#,
        r#"["a",[]]"#,
        r#"[1]"#,
        r#"{"text":"a","extra":[{"text":"b","extra":5}]}"#,
    ];

    #[test]
    fn text_from_json_refuses_objects_that_are_not_components() {
        for refused in NOT_COMPONENTS {
            assert!(component_from_json(refused).is_none(), "{refused}");
        }
        // The same objects in SNBT.
        assert!(component_from_json("{color:red}").is_none());
        assert!(component_from_json("{}").is_none());
    }

    /// Each content kind of `ComponentSerialization`, with `extra` and arrays, is a component.
    #[test]
    fn text_from_json_accepts_each_content_kind() {
        for accepted in [
            r#"{"text":"a"}"#,
            r#"{"text":5}"#,
            r#"{"text":"","extra":[]}"#,
            r#"{"translate":"k"}"#,
            r#"{"translate":"k","fallback":"f","with":["a",1,true,{"text":"b"}]}"#,
            r#"{"keybind":"key.jump"}"#,
            r#"{"selector":"@a","separator":{"text":","}}"#,
            r#"{"score":{"name":"@s","objective":"o"}}"#,
            r#"{"nbt":"a.b","entity":"@s"}"#,
            r#"{"nbt":"a.b","block":"0 64 0","interpret":true}"#,
            r#"{"nbt":"a.b","storage":"ns:s"}"#,
            r#"{"type":"nbt","source":"storage","nbt":"a","storage":"ns:s"}"#,
            r#"{"type":"text","text":"a"}"#,
            r#"{"type":"translatable","translate":"k"}"#,
            r#"{"object":"atlas","sprite":"block/stone"}"#,
            r#"{"player":{"name":"Steve"}}"#,
            r#"{"text":"a","extra":["b",{"text":"c","extra":[{"keybind":"key.use"}]}]}"#,
            r#"["a",{"text":"b"},["c"]]"#,
            "{text:a}",
            "[{text:'a'},'b']",
        ] {
            assert!(component_from_json(accepted).is_some(), "{accepted}");
        }
    }

    /// Through the public host: literal valid components read, literal invalid ones are
    /// `Ok(None)`, and a text over the nesting limit is the named refusal.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn the_host_reads_components_and_returns_none_for_the_rest() {
        use super::super::native_entity_refs::tests::with_game;
        with_game(|game| {
            let host: &dyn mod_api::Host = game;
            let read = |json: &str| host.text_from_json(json).unwrap().map(|t| t.content);
            assert_eq!(
                read(r#"{"text":"Hi"}"#),
                Some(TextContent::Literal("Hi".into()))
            );
            assert_eq!(
                read("\"plain\""),
                Some(TextContent::Literal("plain".into()))
            );
            assert_eq!(
                read(r#"["a",{"text":"b"}]"#),
                Some(TextContent::Literal("a".into()))
            );
            assert!(matches!(
                read(r#"{"translate":"chat.type.text"}"#),
                Some(TextContent::Translatable { .. })
            ));
            for invalid in NOT_COMPONENTS {
                assert_eq!(read(invalid), None, "{invalid}");
            }
            assert_eq!(read("{} trailing"), None);
            assert_eq!(read("12"), None);
        });
    }
}
