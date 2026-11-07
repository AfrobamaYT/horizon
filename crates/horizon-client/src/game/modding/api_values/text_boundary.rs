use super::{TryIntoApi, TryIntoEngine};

impl TryIntoApi for horizon_client_core::text::Text {
    type Api = mod_api::text::Text;
    type Error = mod_api::text::TextFormError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        validate_engine_depth(self)?;
        let component = self.component_view();
        let result = (|| {
            let text = captured_text(&component);
            let form = mod_api::text::TextForm::try_from(&text);
            discard_api_text(text);
            mod_api::text::Text::try_from(form?)
        })();
        if let std::borrow::Cow::Owned(component) = component {
            horizon_client_core::component::discard_component(component);
        }
        result
    }
}

// Conversion temporaries are boundary-owned, not the mod-owned trees whose
// ordinary recursive Clone/Eq/Drop are outside the crossing guarantee.
fn discard_api_text(text: mod_api::Text) {
    use horizon_client_core::component::discard_component;
    use mod_api::text::{TextClick, TextContent as C, TextHover as H, TextObject as O};
    let mut pending = vec![text];
    while let Some(mut text) = pending.pop() {
        pending.append(&mut text.children);
        for value in std::mem::take(&mut text.metadata)
            .into_values()
            .chain(std::mem::take(&mut text.style.metadata).into_values())
        {
            discard_component(value);
        }
        match std::mem::replace(&mut text.content, C::Literal(String::new())) {
            C::Translatable { arguments, .. } => pending.extend(arguments),
            C::Selector { separator, .. } | C::Nbt { separator, .. } => {
                pending.extend(separator.map(|t| *t))
            }
            C::Object(object) => {
                let fallback = match object {
                    O::Player {
                        profile, fallback, ..
                    } => {
                        discard_component(profile);
                        fallback
                    }
                    O::Atlas { fallback, .. } => fallback,
                };
                pending.extend(fallback.map(|t| *t));
            }
            _ => {}
        }
        match text.style.hover.take() {
            Some(H::Text(text)) => pending.push(*text),
            Some(H::Entity { name, .. }) => pending.extend(name.map(|t| *t)),
            Some(H::Item {
                components: Some(components),
                ..
            }) => discard_component(components),
            _ => {}
        }
        match text.style.click.take() {
            Some(TextClick::ShowDialog(data))
            | Some(TextClick::Custom {
                payload: Some(data),
                ..
            }) => discard_component(data),
            _ => {}
        }
    }
}

fn validate_engine_depth(
    text: &horizon_client_core::text::Text,
) -> Result<(), mod_api::text::TextFormError> {
    let mut pending = vec![(text, 1)];
    while let Some((text, depth)) = pending.pop() {
        if depth > mod_api::text::MAX_TEXT_DEPTH {
            return Err(mod_api::text::TextFormError::TooDeep);
        }
        if let Some(source) = text.source_component() {
            validate_component_depth(source, depth)?;
        } else {
            let span_depth = depth + usize::from(text.spans.len() > 1);
            for span in &text.spans {
                if span_depth > mod_api::text::MAX_TEXT_DEPTH {
                    return Err(mod_api::text::TextFormError::TooDeep);
                }
                if let Some(hover) = &span.style.hover {
                    pending.push((hover, span_depth + 1));
                }
                if let Some(entity) = &span.style.hover_entity
                    && let Some(name) = entity.get("name")
                {
                    validate_component_depth(name, span_depth + 1)?;
                }
            }
        }
    }
    Ok(())
}

fn validate_component_depth(
    value: &serde_json::Value,
    depth: usize,
) -> Result<(), mod_api::text::TextFormError> {
    use serde_json::Value;
    let mut pending = vec![(value, depth)];
    while let Some((value, depth)) = pending.pop() {
        if depth > mod_api::text::MAX_TEXT_DEPTH {
            return Err(mod_api::text::TextFormError::TooDeep);
        }
        match value {
            Value::Array(a) => {
                if let Some((first, rest)) = a.split_first() {
                    pending.push((first, depth));
                    pending.extend(rest.iter().map(|v| (v, depth + 1)));
                }
            }
            Value::Object(o) => {
                if let Some(Value::Array(a)) = o.get("extra") {
                    pending.extend(a.iter().map(|v| (v, depth + 1)));
                }
                let kind = o.get("object").and_then(Value::as_str);
                let object = matches!(kind, Some("player") | None) && o.contains_key("player")
                    || matches!(kind, Some("atlas") | None)
                        && o.get("sprite").is_some_and(Value::is_string);
                if object {
                    if let Some(v) = o.get("fallback") {
                        pending.push((v, depth + 1));
                    }
                } else if !o.contains_key("text") {
                    if o.get("translate").is_some_and(Value::is_string) {
                        if let Some(Value::Array(a)) = o.get("with") {
                            pending.extend(a.iter().map(|v| (v, depth + 1)));
                        }
                    } else if o.get("selector").is_some_and(Value::is_string)
                        || o.get("nbt").is_some_and(Value::is_string)
                    {
                        if let Some(v) = o.get("separator") {
                            pending.push((v, depth + 1));
                        }
                    }
                }
                if let Some(e) = horizon_client_core::component::style_event_object(
                    o,
                    "hover_event",
                    "hoverEvent",
                ) {
                    let child = match e.get("action").and_then(Value::as_str) {
                        Some("show_text") => e
                            .get("value")
                            .or_else(|| e.get("contents"))
                            .or_else(|| e.get("text")),
                        Some("show_entity") => e.get("name"),
                        _ => None,
                    };
                    if let Some(v) = child {
                        pending.push((v, depth + 1));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn captured_node<'a>(
    value: &'a serde_json::Value,
    capture: &mut dyn FnMut(&'a serde_json::Value) -> mod_api::text::Text,
) -> mod_api::text::Text {
    use mod_api::text::Text;
    use serde_json::Value;
    match value {
        Value::Object(fields) => {
            let mut metadata = text_metadata(fields).into_iter().collect();
            let mut text = Text::new(captured_text_content(fields, &mut metadata, capture));
            text.style = captured_text_style(fields, capture);
            text.children = fields
                .get("extra")
                .and_then(Value::as_array)
                .map(|children| children.iter().map(&mut *capture).collect())
                .unwrap_or_default();
            text.metadata = metadata;
            text
        }
        Value::Array(values) => {
            let Some((first, rest)) = values.split_first() else {
                return Text::default();
            };
            let mut text = capture(first);
            text.children.extend(rest.iter().map(&mut *capture));
            text
        }
        Value::String(value) => Text::literal(value),
        Value::Null => Text::default(),
        Value::Number(_) | Value::Bool(_) => Text::literal(value.to_string()),
    }
}

fn captured_text_content<'a>(
    fields: &'a serde_json::Map<String, serde_json::Value>,
    metadata: &mut std::collections::BTreeMap<String, serde_json::Value>,
    capture: &mut dyn FnMut(&'a serde_json::Value) -> mod_api::text::Text,
) -> mod_api::text::TextContent {
    use horizon_client_core::{component, text::InlineObject};
    use mod_api::text::{TextContent as C, TextNbtSource as N, TextObject as O};
    use serde_json::Value;
    // Keep structured fallback data; reuse the engine's profile/hat/atlas defaults.
    if let Some(object) = component::inline_object_fields(fields) {
        let fallback = fields.get("fallback").map(|v| Box::new(capture(v)));
        return C::Object(match object {
            InlineObject::Player { profile, hat, .. } => O::Player {
                profile,
                hat,
                fallback,
            },
            InlineObject::Atlas { atlas, sprite, .. } => O::Atlas {
                atlas,
                sprite,
                fallback,
            },
        });
    }
    if let Some(value) = fields.get("text") {
        return C::Literal(
            value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string()),
        );
    }
    if let Some(key) = fields.get("translate").and_then(Value::as_str) {
        return C::Translatable {
            key: key.into(),
            fallback: fields
                .get("fallback")
                .and_then(Value::as_str)
                .map(str::to_owned),
            arguments: fields
                .get("with")
                .and_then(Value::as_array)
                .map(|v| v.iter().map(&mut *capture).collect())
                .unwrap_or_default(),
        };
    }
    if let Some(pattern) = fields.get("selector").and_then(Value::as_str) {
        return C::Selector {
            pattern: pattern.into(),
            separator: fields.get("separator").map(|v| Box::new(capture(v))),
        };
    }
    if let Some(key) = fields.get("keybind").and_then(Value::as_str) {
        return C::Keybind(key.into());
    }
    if let Some(score) = fields.get("score").and_then(Value::as_object)
        && let (Some(name), Some(objective)) = (
            score.get("name").and_then(Value::as_str),
            score.get("objective").and_then(Value::as_str),
        )
    {
        let extra: serde_json::Map<_, _> = score
            .iter()
            .filter(|(key, _)| !["name", "objective"].contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        if !extra.is_empty() {
            metadata.insert("score".into(), Value::Object(extra));
        }
        return C::Score {
            name: name.into(),
            objective: objective.into(),
        };
    }
    if let Some(path) = fields.get("nbt").and_then(Value::as_str) {
        let source = [
            ("block", N::Block as fn(String) -> N),
            ("entity", N::Entity),
            ("storage", N::Storage),
        ]
        .into_iter()
        .find_map(|(field, make)| {
            fields
                .get(field)
                .and_then(Value::as_str)
                .map(|v| make(v.into()))
        });
        if let Some(source) = source {
            return C::Nbt {
                path: path.into(),
                interpret: component::style_flag(fields.get("interpret")).unwrap_or(false),
                source,
                separator: fields.get("separator").map(|v| Box::new(capture(v))),
            };
        }
    }
    C::Literal(String::new())
}

fn captured_click(
    event: &serde_json::Map<String, serde_json::Value>,
) -> Option<mod_api::text::TextClick> {
    use horizon_client_core::{component, text::ClickEvent as E};
    use mod_api::text::TextClick as A;
    Some(match component::parse_click_event(event)? {
        E::OpenUrl(v) => A::OpenUrl(v),
        E::ChangePage(v) => A::ChangePage(v),
        E::SuggestCommand(v) => A::SuggestCommand(v),
        E::RunCommand(v) => A::RunCommand(v),
        E::CopyToClipboard(v) => A::CopyToClipboard(v),
        E::ShowDialog(_) => A::ShowDialog(event["dialog"].clone()),
        E::Custom { id, .. } => A::Custom {
            id,
            payload: event.get("payload").filter(|v| !v.is_null()).cloned(),
        },
        E::OpenFile(_) => return None,
    })
}

fn captured_hover<'a>(
    event: &'a serde_json::Map<String, serde_json::Value>,
    capture: &mut dyn FnMut(&'a serde_json::Value) -> mod_api::text::Text,
) -> Option<mod_api::text::TextHover> {
    use mod_api::text::TextHover as H;
    use serde_json::Value;
    match event.get("action")?.as_str()? {
        "show_text" => Some(H::Text(Box::new(capture(
            event
                .get("value")
                .or_else(|| event.get("contents"))
                .or_else(|| event.get("text"))?,
        )))),
        "show_item" => Some(H::Item {
            id: event.get("id")?.as_str()?.into(),
            count: event
                .get("count")
                .and_then(Value::as_u64)
                .and_then(|v| u32::try_from(v).ok())
                .unwrap_or(1),
            components: event.get("components").cloned(),
        }),
        "show_entity" => {
            let uuid = event.get("uuid")?;
            let words = if let Some(text) = uuid.as_str() {
                horizon_client_core::uuid::uuid_from_text(text)?
            } else {
                let values: &[Value; 4] = uuid.as_array()?.as_slice().try_into().ok()?;
                let mut words = [0; 4];
                for (out, value) in words.iter_mut().zip(values) {
                    *out = i32::try_from(value.as_i64()?).ok()?;
                }
                words
            }
            .map(|word| word as u32 as u64);
            Some(H::Entity {
                kind: event.get("id")?.as_str()?.into(),
                id: [words[0] << 32 | words[1], words[2] << 32 | words[3]],
                name: event.get("name").map(|v| Box::new(capture(v))),
            })
        }
        _ => None,
    }
}

fn captured_text_style<'a>(
    fields: &'a serde_json::Map<String, serde_json::Value>,
    capture: &mut dyn FnMut(&'a serde_json::Value) -> mod_api::text::Text,
) -> mod_api::text::TextStyle {
    use horizon_client_core::component;
    use serde_json::Value;
    let mut style = mod_api::text::TextStyle::default();
    style.color = fields
        .get("color")
        .and_then(Value::as_str)
        .map(str::to_owned);
    style.shadow_color = fields.get("shadow_color").and_then(component::shadow_color);
    style.bold = component::style_flag(fields.get("bold"));
    style.italic = component::style_flag(fields.get("italic"));
    style.underlined = component::style_flag(fields.get("underlined"));
    style.strikethrough = component::style_flag(fields.get("strikethrough"));
    style.obfuscated = component::style_flag(fields.get("obfuscated"));
    style.font = fields
        .get("font")
        .and_then(Value::as_str)
        .map(str::to_owned);
    style.insertion = fields
        .get("insertion")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let click = component::style_event_object(fields, "click_event", "clickEvent");
    let hover = component::style_event_object(fields, "hover_event", "hoverEvent");
    style.click = click.and_then(captured_click);
    style.hover = hover.and_then(|event| captured_hover(event, capture));
    for (field, event) in [
        ("click_event", click.filter(|_| style.click.is_some())),
        ("hover_event", hover.filter(|_| style.hover.is_some())),
    ] {
        if let Some(event) = event {
            let metadata = text_event_metadata(event);
            if !metadata.is_empty() {
                style.metadata.insert(field.into(), Value::Object(metadata));
            }
        }
    }
    style
}

impl<C: ?Sized> TryIntoEngine<C> for mod_api::text::Text {
    type Engine = horizon_client_core::text::Text;
    type Error = mod_api::text::TextFormError;
    fn try_into_engine(&self, _: &C) -> Result<Self::Engine, Self::Error> {
        let form = mod_api::text::TextForm::try_from(self)?;
        let text = mod_api::text::Text::try_from(form)?;
        let component = owned_text_component(&text);
        discard_api_text(text);
        Ok(component.into())
    }
}

fn text_metadata<'a>(
    metadata: impl IntoIterator<Item = (&'a String, &'a serde_json::Value)>,
) -> serde_json::Map<String, serde_json::Value> {
    let reserved = [
        "type",
        "text",
        "translate",
        "with",
        "fallback",
        "keybind",
        "selector",
        "separator",
        "score",
        "nbt",
        "interpret",
        "block",
        "entity",
        "storage",
        "object",
        "player",
        "hat",
        "atlas",
        "sprite",
        "extra",
        "color",
        "shadow_color",
        "bold",
        "italic",
        "underlined",
        "strikethrough",
        "obfuscated",
        "font",
        "insertion",
        "click_event",
        "hover_event",
        "clickEvent",
        "hoverEvent",
    ];
    metadata
        .into_iter()
        .filter(|(key, _)| !reserved.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn owned_node<'a>(
    text: &'a mod_api::text::Text,
    emit: &mut dyn FnMut(&'a mod_api::text::Text) -> serde_json::Value,
) -> serde_json::Value {
    use mod_api::text::{TextContent as C, TextNbtSource as N, TextObject as O};
    use serde_json::Value;
    let mut out = text_metadata(&text.metadata);
    match &text.content {
        C::Literal(value) => {
            out.insert("text".into(), value.clone().into());
        }
        C::Translatable {
            key,
            fallback,
            arguments,
        } => {
            out.insert("translate".into(), key.clone().into());
            if let Some(fallback) = fallback {
                out.insert("fallback".into(), fallback.clone().into());
            }
            if !arguments.is_empty() {
                out.insert(
                    "with".into(),
                    Value::Array(
                        arguments
                            .iter()
                            .map(|arg| horizon_client_core::text::argument_component(emit(arg)))
                            .collect(),
                    ),
                );
            }
        }
        C::Keybind(value) => {
            out.insert("keybind".into(), value.clone().into());
        }
        C::Selector { pattern, separator } => {
            out.insert("selector".into(), pattern.clone().into());
            if let Some(separator) = separator {
                out.insert("separator".into(), emit(separator));
            }
        }
        C::Score { name, objective } => {
            let mut score = text
                .metadata
                .get("score")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            score.insert("name".into(), name.clone().into());
            score.insert("objective".into(), objective.clone().into());
            out.insert("score".into(), Value::Object(score));
        }
        C::Nbt {
            path,
            interpret,
            source,
            separator,
        } => {
            out.insert("nbt".into(), path.clone().into());
            out.insert("interpret".into(), (*interpret).into());
            let (key, value) = match source {
                N::Block(v) => ("block", v),
                N::Entity(v) => ("entity", v),
                N::Storage(v) => ("storage", v),
            };
            out.insert(key.into(), value.clone().into());
            if let Some(separator) = separator {
                out.insert("separator".into(), emit(separator));
            }
        }
        C::Object(object) => {
            let fallback = match object {
                O::Player {
                    profile,
                    hat,
                    fallback,
                } => {
                    out.extend([
                        ("object".into(), "player".into()),
                        ("player".into(), profile.clone()),
                        ("hat".into(), (*hat).into()),
                    ]);
                    fallback
                }
                O::Atlas {
                    atlas,
                    sprite,
                    fallback,
                } => {
                    out.extend([
                        ("object".into(), "atlas".into()),
                        ("atlas".into(), atlas.clone().into()),
                        ("sprite".into(), sprite.clone().into()),
                    ]);
                    fallback
                }
            };
            if let Some(fallback) = fallback {
                out.insert("fallback".into(), emit(fallback));
            }
        }
    }
    out.extend(owned_text_style(&text.style, emit));
    if !text.children.is_empty() {
        out.insert(
            "extra".into(),
            text.children.iter().map(&mut *emit).collect(),
        );
    }
    Value::Object(out)
}

fn text_action_metadata(
    style: &mod_api::text::TextStyle,
    field: &str,
) -> serde_json::Map<String, serde_json::Value> {
    style
        .metadata
        .get(field)
        .and_then(serde_json::Value::as_object)
        .map(text_event_metadata)
        .unwrap_or_default()
}

fn text_event_metadata(
    event: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    let reserved = [
        "action",
        "url",
        "page",
        "command",
        "value",
        "contents",
        "dialog",
        "id",
        "payload",
        "path",
        "count",
        "components",
        "uuid",
        "name",
        "text",
    ];
    event
        .iter()
        .filter(|(key, _)| !reserved.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn text_action(
    action: &str,
    field: &str,
    value: serde_json::Value,
) -> serde_json::Map<String, serde_json::Value> {
    [("action".into(), action.into()), (field.into(), value)]
        .into_iter()
        .collect()
}

fn owned_text_style<'a>(
    style: &'a mod_api::text::TextStyle,
    emit: &mut dyn FnMut(&'a mod_api::text::Text) -> serde_json::Value,
) -> serde_json::Map<String, serde_json::Value> {
    use mod_api::text::{TextClick as C, TextHover as H};
    use serde_json::Value;
    let mut out = text_metadata(&style.metadata);
    for (key, value) in [
        ("color", style.color.clone().map(Value::String)),
        (
            "shadow_color",
            style.shadow_color.map(|c| Value::from(c as i32)),
        ),
        ("bold", style.bold.map(Value::Bool)),
        ("italic", style.italic.map(Value::Bool)),
        ("underlined", style.underlined.map(Value::Bool)),
        ("strikethrough", style.strikethrough.map(Value::Bool)),
        ("obfuscated", style.obfuscated.map(Value::Bool)),
        ("font", style.font.clone().map(Value::String)),
        ("insertion", style.insertion.clone().map(Value::String)),
    ] {
        if let Some(value) = value {
            out.insert(key.into(), value);
        }
    }
    if let Some(click) = &style.click {
        let fields = match click {
            C::OpenUrl(v) => text_action("open_url", "url", v.clone().into()),
            C::ChangePage(v) => text_action("change_page", "page", (*v).into()),
            C::SuggestCommand(v) => text_action("suggest_command", "command", v.clone().into()),
            C::RunCommand(v) => text_action("run_command", "command", v.clone().into()),
            C::CopyToClipboard(v) => text_action("copy_to_clipboard", "value", v.clone().into()),
            C::ShowDialog(v) => text_action("show_dialog", "dialog", v.clone()),
            C::Custom { id, payload } => {
                let mut fields = text_action("custom", "id", id.clone().into());
                if let Some(payload) = payload {
                    fields.insert("payload".into(), payload.clone());
                }
                fields
            }
        };
        let mut action = text_action_metadata(style, "click_event");
        action.extend(fields);
        out.insert("click_event".into(), Value::Object(action));
    }
    if let Some(hover) = &style.hover {
        let mut action = text_action_metadata(style, "hover_event");
        match hover {
            H::Text(value) => {
                action.insert("action".into(), "show_text".into());
                action.insert("value".into(), emit(value));
            }
            H::Item {
                id,
                count,
                components,
            } => {
                action.insert("action".into(), "show_item".into());
                action.insert("id".into(), id.clone().into());
                action.insert("count".into(), (*count).into());
                if let Some(components) = components {
                    action.insert("components".into(), components.clone());
                }
            }
            H::Entity { kind, id, name } => {
                action.insert("action".into(), "show_entity".into());
                action.insert("id".into(), kind.clone().into());
                action.insert(
                    "uuid".into(),
                    horizon_client_core::uuid::uuid_text([
                        (id[0] >> 32) as i32,
                        id[0] as i32,
                        (id[1] >> 32) as i32,
                        id[1] as i32,
                    ])
                    .into(),
                );
                if let Some(name) = name {
                    action.insert("name".into(), emit(name));
                }
            }
        }
        out.insert("hover_event".into(), Value::Object(action));
    }
    out
}

// Evaluate a tree bottom-up without cloning any recursive edge. The shallow
// visitor owns field semantics; this work list owns traversal for both directions.
fn postorder<T, R>(
    root: &T,
    mut node: impl for<'a> FnMut(&'a T, &mut dyn FnMut(&'a T) -> R) -> R,
    empty: impl Fn() -> R,
) -> R {
    let mut pending = vec![(root, false)];
    let mut ready = std::collections::HashMap::new();
    while let Some((value, visited)) = pending.pop() {
        let key = std::ptr::from_ref(value) as usize;
        if visited {
            let result = node(value, &mut |child| {
                ready
                    .remove(&(std::ptr::from_ref(child) as usize))
                    .expect("child visited before parent")
            });
            ready.insert(key, result);
        } else {
            pending.push((value, true));
            let mut children = Vec::new();
            drop(node(value, &mut |child| {
                children.push(child);
                empty()
            }));
            pending.extend(children.into_iter().rev().map(|child| (child, false)));
        }
    }
    ready
        .remove(&(std::ptr::from_ref(root) as usize))
        .expect("root visited")
}
pub(super) fn captured_text(value: &serde_json::Value) -> mod_api::text::Text {
    postorder(value, captured_node, mod_api::text::Text::default)
}
fn owned_text_component(text: &mod_api::text::Text) -> serde_json::Value {
    postorder(text, owned_node, || serde_json::Value::Null)
}

#[cfg(test)]
mod text_capture_tests {
    use super::{TryIntoApi, TryIntoEngine};

    #[test]
    fn native_translation_fallback_survives_noop_owned_round_trip() {
        let language = horizon_client_core::lang::Language::of(
            "en_us",
            [("test.fallback".into(), "Wrong translation".into())],
        );
        horizon_client_core::lang::scoped(language, || {
            let native = horizon_client_core::text::Text::translate_with_fallback(
                "test.missing",
                "test.fallback",
                &[],
            );
            let mut api = native.try_into_api().unwrap();
            let listener = |_: &mut mod_api::text::Text| {};
            listener(&mut api);
            let actual = api.try_into_engine(&()).unwrap();
            assert_eq!(actual.plain(), "test.fallback");
            assert_eq!(actual.spans, native.spans);
            assert_eq!(actual.component(), native.component());
        });
    }
    use horizon_client_core::text::Text as EngineText;
    use mod_api::text::{TextContent as C, TextHover};
    use serde_json::json;

    #[test]
    fn untrusted_file_action_cannot_become_local_after_a_noop_value_round_trip() {
        let source = json!({"text":"server", "click_event":{"action":"open_file", "path":"test-file"},
            "extra":[{"text":"child", "clickEvent":{"action":"open_file", "value":"test-file"}}]});
        let mut api = EngineText::from(source).try_into_api().unwrap();
        assert!(api.style.click.is_none());
        assert!(!api.metadata.contains_key("click_event"));
        assert!(!api.style.metadata.contains_key("click_event"));
        assert!(api.children[0].style.click.is_none());
        let listener = |_: &mut mod_api::text::Text| {};
        listener(&mut api);
        let engine = api.try_into_engine(&()).unwrap();
        assert!(engine.spans.iter().all(|span| span.style.click.is_none()));
        let component = engine.component();
        assert!(component.get("click_event").is_none());
        assert!(component["extra"][0].get("clickEvent").is_none());
    }

    #[test]
    fn component_tree_capture_keeps_dynamic_contents_and_unknown_metadata() {
        let source = json!({"translate":"test.message", "fallback":"%s", "with":[{"text":"name", "color":"red"}],
            "bold":false, "test:meta":{"n":null}, "click_event":{"action":"custom", "id":"test:open", "payload":{"n":null}, "test:trace":true},
            "extra":[{"nbt":"value", "storage":"test:data", "interpret":true, "separator":{"text":","}},
                {"score":{"name":"@s", "objective":"test", "test:trace":2}}]});
        let api = EngineText::from(source.clone()).try_into_api().unwrap();
        assert!(
            matches!(&api.content, C::Translatable { key, fallback:Some(fallback), arguments } if key == "test.message" && fallback == "%s" && arguments.len() == 1)
        );
        assert_eq!(api.style.bold, Some(false));
        assert!(
            matches!(&api.children[0].content, C::Nbt { path, interpret:true, .. } if path == "value")
        );
        assert_eq!(
            api.style.metadata["click_event"],
            json!({"test:trace":true})
        );
        assert_eq!(api.try_into_engine(&()).unwrap().component(), source);
    }

    #[test]
    fn object_fallback_round_trip_keeps_omitted_styled_and_translatable() {
        for object in [
            json!({"object":"player", "player":{"name":"Test", "properties":[{"name":"textures", "value":"data", "signature":"signed"}]}, "hat":true}),
            json!({"object":"atlas", "atlas":"minecraft:blocks", "sprite":"test:icon"}),
        ] {
            for fallback in [
                None,
                Some(
                    json!({"text":"label", "italic":false, "extra":[{"text":"child", "bold":true}]}),
                ),
                Some(json!({"translate":"test.head", "with":[{"text":"name"}]})),
            ] {
                let mut source = object.clone();
                if let Some(fallback) = fallback {
                    source["fallback"] = fallback;
                }
                let api = EngineText::from(source.clone()).try_into_api().unwrap();
                assert_eq!(api.try_into_engine(&()).unwrap().component(), source);
            }
        }
    }

    #[test]
    fn hover_uuid_forms_keep_identity_and_unresolved_name() {
        for uuid in [
            json!("11223344-5566-7788-99aa-bbccddeeff00"),
            json!([287454020, 1432778632, -1716864052_i64, -571539712]),
        ] {
            let source = json!({"text":"entity", "hover_event":{"action":"show_entity", "id":"minecraft:pig", "uuid":uuid, "name":{"translate":"test.name"}, "test:trace":true}});
            let api = EngineText::from(source).try_into_api().unwrap();
            let Some(TextHover::Entity {
                id,
                name: Some(name),
                ..
            }) = &api.style.hover
            else {
                panic!("entity hover")
            };
            assert_eq!(*id, [0x1122334455667788, 0x99aabbccddeeff00]);
            assert!(matches!(&name.content, C::Translatable { key, .. } if key == "test.name"));
            assert_eq!(
                api.try_into_engine(&()).unwrap().component()["hover_event"],
                json!({"action":"show_entity", "id":"minecraft:pig", "uuid":"11223344-5566-7788-99aa-bbccddeeff00", "name":{"translate":"test.name"}, "test:trace":true})
            );
        }
    }

    #[test]
    fn style_capture_reuses_snbt_flags_and_rgba_shadow_rules() {
        let source = json!({"text":"styled", "bold":0, "italic":1, "shadow_color":[1,0,0.5,1]});
        let api = EngineText::from(source).try_into_api().unwrap();
        assert_eq!(api.style.bold, Some(false));
        assert_eq!(api.style.italic, Some(true));
        assert_eq!(api.style.shadow_color, Some(0xffff007f));
    }
}

#[cfg(test)]
mod text_encoding_tests {
    use super::TryIntoEngine;
    use mod_api::text::{Text, TextClick, TextContent as C, TextHover, TextNbtSource, TextObject};
    use serde_json::json;

    #[test]
    fn owned_text_keeps_unresolved_translation_style_and_ordered_children() {
        let mut text = Text::translate("test.message", vec![Text::literal("7")]);
        text.style.bold = Some(false);
        text.style.color = Some("red".into());
        text.style.shadow_color = Some(0xff001122);
        text.style.click = Some(TextClick::RunCommand("/test".into()));
        text.style
            .metadata
            .insert("click_event".into(), json!({"test:extra":true}));
        text.metadata
            .insert("test:context".into(), json!({"value":null}));
        text.children.push(Text::literal("first"));
        text.children.push(Text::literal("second"));
        let engine: Result<horizon_client_core::text::Text, mod_api::text::TextFormError> =
            text.try_into_engine(&());
        assert_eq!(
            engine.unwrap().component(),
            json!({
                "translate":"test.message", "with":[7], "bold":false, "color":"red",
                "shadow_color":-16772830, "click_event":{"action":"run_command", "command":"/test", "test:extra":true},
                "test:context":{"value":null}, "extra":[{"text":"first"}, {"text":"second"}]
            })
        );
    }

    #[test]
    fn owned_text_retains_dynamic_sources_and_inline_profile_properties() {
        for (content, expected) in [
            (C::Keybind("key.jump".into()), json!({"keybind":"key.jump"})),
            (
                C::Selector {
                    pattern: "@p".into(),
                    separator: Some(Box::new(Text::literal(","))),
                },
                json!({"selector":"@p", "separator":{"text":","}}),
            ),
            (
                C::Score {
                    name: "@s".into(),
                    objective: "test".into(),
                },
                json!({"score":{"name":"@s", "objective":"test"}}),
            ),
            (
                C::Nbt {
                    path: "value".into(),
                    interpret: true,
                    source: TextNbtSource::Storage("test:data".into()),
                    separator: None,
                },
                json!({"nbt":"value", "interpret":true, "storage":"test:data"}),
            ),
            (
                C::Object(TextObject::Player {
                    profile: json!({"name":"Test", "properties":[{"name":"textures", "value":"data", "signature":"signed"}]}),
                    hat: false,
                    fallback: Some(Box::new(Text::literal("head"))),
                }),
                json!({"object":"player", "player":{"name":"Test", "properties":[{"name":"textures", "value":"data", "signature":"signed"}]}, "hat":false, "fallback":{"text":"head"}}),
            ),
        ] {
            let engine = Text::new(content).try_into_engine(&()).unwrap();
            assert_eq!(engine.component(), expected);
        }
    }

    #[test]
    fn object_fallback_encoder_keeps_default_styling_and_translation_tree() {
        let mut styled = Text::literal("styled");
        styled.style.italic = Some(false);
        for player in [true, false] {
            for (fallback, expected) in [
                (None, None),
                (
                    Some(Box::new(styled.clone())),
                    Some(json!({"text":"styled", "italic":false})),
                ),
                (
                    Some(Box::new(Text::translate("test.head", vec![]))),
                    Some(json!({"translate":"test.head"})),
                ),
            ] {
                let object = if player {
                    TextObject::Player {
                        profile: json!({"name":"Test"}),
                        hat: true,
                        fallback,
                    }
                } else {
                    TextObject::Atlas {
                        atlas: "minecraft:blocks".into(),
                        sprite: "test:icon".into(),
                        fallback,
                    }
                };
                let engine = Text::new(C::Object(object)).try_into_engine(&()).unwrap();
                match expected {
                    None => {
                        assert!(engine.component().get("fallback").is_none());
                        assert_eq!(
                            engine.plain(),
                            if player { "[Test head]" } else { "[test:icon]" }
                        );
                    }
                    Some(expected) => assert_eq!(engine.component()["fallback"], expected),
                }
            }
        }
    }

    #[test]
    fn raw_file_action_metadata_does_not_reach_the_engine() {
        let mut text = Text::literal("remote");
        for metadata in [&mut text.metadata, &mut text.style.metadata] {
            for field in ["click_event", "clickEvent"] {
                metadata.insert(
                    field.into(),
                    json!({"action":"open_file", "path":"test-file"}),
                );
            }
        }
        let engine = text.try_into_engine(&()).unwrap();
        assert!(engine.spans.iter().all(|span| span.style.click.is_none()));
        assert!(engine.component().get("click_event").is_none());
        assert!(engine.component().get("clickEvent").is_none());
    }

    #[test]
    fn owned_text_keeps_hover_uuid_name_and_nullable_component_patch() {
        let mut text = Text::literal("entity");
        text.style.hover = Some(TextHover::Entity {
            kind: "minecraft:pig".into(),
            id: [0x1122334455667788, 0x99aabbccddeeff00],
            name: Some(Box::new(Text::literal("name"))),
        });
        let engine = text.try_into_engine(&()).unwrap();
        assert_eq!(
            engine.component()["hover_event"],
            json!({"action":"show_entity", "id":"minecraft:pig", "uuid":"11223344-5566-7788-99aa-bbccddeeff00", "name":{"text":"name"}})
        );
        text.style.hover = Some(TextHover::Item {
            id: "minecraft:stone".into(),
            count: 3,
            components: Some(json!({"minecraft:custom_name":null})),
        });
        let engine = text.try_into_engine(&()).unwrap();
        assert_eq!(
            engine.component()["hover_event"],
            json!({"action":"show_item", "id":"minecraft:stone", "count":3, "components":{"minecraft:custom_name":null}})
        );
    }
}

#[cfg(test)]
mod iterative_tests {
    use super::*;
    use mod_api::text::TextForm;
    use serde_json::json;

    #[test]
    fn recursive_fields_survive_512_nodes_on_256_kib() {
        std::thread::Builder::new().stack_size(256 * 1024).spawn(|| {
            for (relation, depth) in (0..9).flat_map(|r| [(r, 512), (r, 513)]) {
                let mut source = json!({"text":"leaf", "bold":false, "test:metadata":17});
                for _ in 1..depth {
                    let mut parent = match relation {
                        0 => json!({"text":"parent"}),
                        1 => json!({"translate":"test:translation", "fallback":"%s"}),
                        2 => json!({"selector":"@s"}),
                        3 => json!({"object":"atlas", "sprite":"test:icon"}),
                        6 => json!({"nbt":"name", "storage":"test:data", "interpret":true}),
                        7 => json!({"object":"player", "player":{"name":"test"}}),
                        8 => json!({"text":""}),
                        _ => json!({"text":"parent"}),
                    };
                    let fields = parent.as_object_mut().unwrap();
                    match relation {
                        0 | 1 => { fields.insert(if relation == 0 { "extra" } else { "with" }.into(), serde_json::Value::Array(vec![source])); }
                        2 | 3 | 6 | 7 => { fields.insert(if relation == 2 || relation == 6 { "separator" } else { "fallback" }.into(), source); }
                        _ => {
                            let mut hover = if relation == 4 { json!({"action":"show_text"}) } else { json!({"action":"show_entity", "id":"minecraft:pig", "uuid":[0,0,0,1]}) };
                            hover.as_object_mut().unwrap().insert(if relation == 4 { "value" } else { "name" }.into(), source);
                            fields.insert("hover_event".into(), hover);
                        }
                    }
                    source = parent;
                }
                let engine = horizon_client_core::text::Text::empty().with_component(source);
                if depth == 513 {
                    assert!(matches!(engine.try_into_api(), Err(mod_api::text::TextFormError::TooDeep)));
                    let api = captured_text(&engine.component_view());
                    assert!(matches!(api.try_into_engine(&()), Err(mod_api::text::TextFormError::TooDeep)));
                    std::mem::forget((engine, api));
                    continue;
                }
                let api = engine.try_into_api().unwrap();
                let expected = TextForm::try_from(&api).unwrap();
                assert_eq!(expected.nodes.len(), 512);
                assert!(expected.nodes.last().unwrap().data.contains("leaf"));
                let emitted = api.try_into_engine(&()).unwrap();
                let restored = emitted.try_into_api().unwrap();
                let actual = TextForm::try_from(&restored).unwrap();
                assert_eq!(actual, expected);
                // Recursive ownership operations are not the crossing guarantee.
                std::mem::forget((engine, api, emitted, restored));
            }
        }).unwrap().join().unwrap();
    }

    #[test]
    fn too_deep_is_refused_before_cloning_opaque_metadata() {
        // Keep opaque metadata much deeper than the small stack could recursively clone.
        // The component-depth refusal must win before any metadata is copied.
        let mut opaque = serde_json::Value::Null;
        for _ in 0..4096 {
            opaque = serde_json::Value::Array(vec![opaque]);
        }
        let mut api = mod_api::Text::literal("leaf");
        for _ in 1..513 {
            api = mod_api::Text::literal("parent").append(api);
        }
        // Build the source separately without serde's recursive json! value serializer.
        let mut component = json!({"text":"leaf"});
        for _ in 1..513 {
            let mut parent = serde_json::Map::new();
            parent.insert("text".into(), "parent".into());
            parent.insert("extra".into(), serde_json::Value::Array(vec![component]));
            component = serde_json::Value::Object(parent);
        }
        api.metadata.insert("test:opaque".into(), opaque);
        // A separate opaque value avoids cloning either deep input in the test itself.
        let mut opaque = serde_json::Value::Null;
        for _ in 0..4096 {
            opaque = serde_json::Value::Array(vec![opaque]);
        }
        component
            .as_object_mut()
            .unwrap()
            .insert("test:opaque".into(), opaque);
        let engine = horizon_client_core::text::Text::empty().with_component(component);
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(move || {
                assert!(matches!(
                    api.try_into_engine(&()),
                    Err(mod_api::text::TextFormError::TooDeep)
                ));
                assert!(matches!(
                    engine.try_into_api(),
                    Err(mod_api::text::TextFormError::TooDeep)
                ));
                std::mem::forget((api, engine));
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn span_only_hover_sources_are_validated_and_materialized_iteratively() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                use horizon_client_core::text::{Span, Style, Text as Engine};
                for depth in [512, 513] {
                    let mut text = Engine::empty();
                    text.spans.push(Span {
                        text: "leaf".into(),
                        style: Style::default(),
                    });
                    for _ in 1..depth {
                        let mut parent = Engine::empty();
                        parent.spans.push(Span {
                            text: "parent".into(),
                            style: Style {
                                hover: Some(std::sync::Arc::new(text)),
                                ..Style::default()
                            },
                        });
                        text = parent;
                    }
                    let api = text.try_into_api();
                    if depth == 513 {
                        assert!(matches!(api, Err(mod_api::text::TextFormError::TooDeep)));
                    } else {
                        assert_eq!(
                            TextForm::try_from(api.as_ref().unwrap())
                                .unwrap()
                                .nodes
                                .len(),
                            512
                        );
                    }
                    std::mem::forget((text, api));
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn array_root_and_children_keep_order_and_metadata() {
        let source = json!([
            {"text":"root", "extra":[{"text":"first"}], "bold":false},
            {"text":"second", "test:metadata":{"id":7}}
        ]);
        let api = captured_text(&source);
        assert_eq!(api.children.len(), 2);
        assert_eq!(
            owned_text_component(&api),
            json!({
                "text":"root", "bold":false,
                "extra":[{"text":"first"}, {"text":"second", "test:metadata":{"id":7}}]
            })
        );
    }

    #[test]
    fn component_view_borrows_retained_source() {
        let engine =
            horizon_client_core::text::Text::empty().with_component(json!({"text":"source"}));
        assert!(matches!(
            engine.component_view(),
            std::borrow::Cow::Borrowed(_)
        ));
    }
}
