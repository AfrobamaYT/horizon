//! The argument kinds a native command usage names: the built-in ones, each a Brigadier
//! parser, and the types mods registered, with the dispatcher type each node parses as.

use super::{ArgType, Text, TryIntoEngine};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};

/// The argument kinds a native usage may name, each with a [`parser`]; `resource:<registry>`
/// and `mod_id` come on top.
pub(super) const KINDS: [&str; 11] = [
    "word",
    "text",
    "integer",
    "number",
    "boolean",
    "player",
    "entity",
    "entities",
    "players",
    "item",
    "block_pos",
];

/// The custom argument types the mods registered, by namespaced id, for the command tree being
/// built and its server dispatcher.
pub(in super::super) type CommandArguments = HashMap<String, Arc<dyn mod_api::CommandArgumentType>>;

/// The argument types the installed mods registered at startup, and the built-in `mod_id`:
/// NeoForge's `ModIdArgument`, one of the loaded package ids, delivered as a string.
pub(in super::super) fn command_arguments() -> CommandArguments {
    let mod_ids = mod_api::__private::loaded_mods()
        .ids()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mod_id: Arc<dyn mod_api::CommandArgumentType> =
        Arc::new(mod_api::ChoiceArgument::new(mod_ids));
    mod_api::__private::command_argument_types()
        .into_iter()
        .chain([(MOD_ID.to_owned(), mod_id)])
        .collect()
}

/// The built-in kind whose value is a loaded package id. Mods' kinds are namespaced, so it
/// never meets one.
pub(super) const MOD_ID: &str = "mod_id";

/// The brigadier parser of an argument kind, its properties and its dispatcher type.
/// `EntityArgument.entity` / `entities` / `player` / `players` share the vanilla parser.
/// `resource:<registry>` is `ResourceKeyArgument` on the wire, so vanilla clients complete it, and
/// `ResourceArgument` on the server, which refuses an id the registry lacks before the mod runs.
pub(super) fn parser(kind: &str) -> Option<(&'static str, Value, ArgType)> {
    if let Some(registry) = kind.strip_prefix("resource:") {
        let registry = horizon_client::command::network::intern(registry);
        return registry.contains(':').then(|| {
            (
                "minecraft:resource_key",
                json!({ "registry": registry }),
                ArgType::Res(registry),
            )
        });
    }
    Some(match kind {
        "word" => ("brigadier:string", json!({"type": "word"}), ArgType::Word),
        "text" => (
            "brigadier:string",
            json!({"type": "greedy"}),
            ArgType::Greedy,
        ),
        "integer" => (
            "brigadier:integer",
            Value::Null,
            ArgType::Integer {
                min: i32::MIN,
                max: i32::MAX,
            },
        ),
        "number" => (
            "brigadier:double",
            Value::Null,
            ArgType::Double {
                min: f64::MIN,
                max: f64::MAX,
            },
        ),
        "boolean" => ("brigadier:bool", Value::Null, ArgType::Bool),
        "entity" | "entities" | "player" | "players" => {
            let single = matches!(kind, "entity" | "player");
            let players = matches!(kind, "player" | "players");
            (
                "minecraft:entity",
                json!({
                    "amount": if single { "single" } else { "multiple" },
                    "type": if players { "players" } else { "entities" },
                }),
                ArgType::Entity { single, players },
            )
        }
        "item" => ("minecraft:item_stack", Value::Null, ArgType::Item),
        "block_pos" => ("minecraft:block_pos", Value::Null, ArgType::BlockPos),
        _ => return None,
    })
}

/// The message a mod's argument parser refused with, as the engine's text. A text the
/// boundary cannot carry is reported as its own diagnostic: the refusal still fails the
/// argument, never silently passing it.
fn refused_argument_text(error: mod_api::CommandArgumentError) -> Text {
    let message: mod_api::Text = error.into();
    match message.try_into_engine(&()) {
        Ok(text) => text,
        Err(error) => Text::literal(error.to_string()),
    }
}

/// The dispatcher type of one validated tree node: a vanilla kind, or the mod's registered
/// argument type with its parser and suggestions.
pub(super) fn argument_type(node: &Value, arguments: &CommandArguments) -> ArgType {
    if node["parser"] == "minecraft:resource_key" {
        let registry = node["properties"]["registry"].as_str().unwrap_or_default();
        return ArgType::Res(horizon_client::command::network::intern(registry));
    }
    if let Some(id) = node["custom_argument"].as_str() {
        let argument = arguments
            .get(id)
            .unwrap_or_else(|| panic!("native command tree names unregistered argument type {id}"));
        let parse = argument.clone();
        let suggest = argument.clone();
        // Use the validated tree property rather than calling mod code again.
        let kind = if node["properties"]["type"] == "greedy" {
            horizon_client::command::CustomArgumentKind::Greedy
        } else {
            horizon_client::command::CustomArgumentKind::Word
        };
        return ArgType::Custom(horizon_client::command::CustomArgument::new(
            id,
            kind,
            move |input| {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse.parse(input)))
                    .map_err(|_| Text::tr("commands.generic.failed", &[]))?
                    .map_err(refused_argument_text)
            },
            move |prefix| {
                let Ok(results) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    suggest.suggest(prefix)
                })) else {
                    eprintln!("mod command argument completer panicked");
                    return Vec::new();
                };
                // Suggestions use STRING_UTF8 (Java UTF-16 length). Tooltips are
                // literal NBT strings, whose modified UTF-8 byte length is unsigned u16.
                if results.iter().any(|s| {
                    s.text.encode_utf16().count() > 32767
                        || s.tooltip.as_ref().is_some_and(|tip| {
                            tip.encode_utf16()
                                .map(|unit| match unit {
                                    1..=0x7f => 1,
                                    0..=0x7ff => 2,
                                    _ => 3,
                                })
                                .sum::<usize>()
                                > 65535
                        })
                }) {
                    eprintln!("mod command argument completion exceeds the protocol string limit");
                    return Vec::new();
                }
                results
                    .into_iter()
                    .map(|suggestion| horizon_client::command::CustomSuggestion {
                        text: suggestion.text,
                        tooltip: suggestion.tooltip,
                    })
                    .collect()
            },
        ));
    }
    KINDS
        .into_iter()
        .filter_map(parser)
        .find_map(|(id, properties, ty)| {
            (node["parser"] == id && node["properties"] == properties).then_some(ty)
        })
        .expect("native command tree contains registered parsers")
}
