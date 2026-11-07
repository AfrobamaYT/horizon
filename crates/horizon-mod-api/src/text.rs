//! Owned component trees. No language is resolved until the game displays a component.
use serde::{Deserialize, Serialize};

/// A component tree, retaining translation keys, styles and ordered children.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[horizon_mod_api_macros::wasm(via = TextForm)]
#[non_exhaustive]
pub struct Text {
    /// The component's contents before its children.
    pub content: TextContent,
    /// Fields not set here inherit from the enclosing component.
    pub style: TextStyle,
    /// Components appended in order.
    pub children: Vec<Text>,
    /// Unrecognised component metadata survives an engine round trip.
    #[serde(default)]
    pub metadata: std::collections::BTreeMap<String, serde_json::Value>,
}

/// What supplies the characters or inline object of a component.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// exhaustive: closed serialized component vocabulary; new cases change the link contract
pub enum TextContent {
    /// Literal characters, without interpreting legacy formatting codes.
    Literal(String),
    /// A translation remains a key and components, not a localized string.
    Translatable {
        /// Translation identifier.
        key: String,
        /// Template used when the key is not present in the language.
        fallback: Option<String>,
        /// Ordered `%s` arguments, retaining their own styles.
        arguments: Vec<Text>,
    },
    /// A key binding's localized display name.
    Keybind(String),
    /// A selector resolved by the game's component context.
    Selector {
        /// Selector expression.
        pattern: String,
        /// Component between the selected names.
        separator: Option<Box<Text>>,
    },
    /// A scoreboard value resolved by the game's component context.
    Score {
        /// Score holder or selector.
        name: String,
        /// Scoreboard objective.
        objective: String,
    },
    /// NBT resolved by the game's component context.
    Nbt {
        /// NBT path expression.
        path: String,
        /// Whether the selected strings contain components.
        interpret: bool,
        /// Where to obtain the NBT.
        source: TextNbtSource,
        /// Component between selected values.
        separator: Option<Box<Text>>,
    },
    /// A non-character inline object.
    Object(TextObject),
}

/// The source of an NBT component, not the NBT itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
// exhaustive: closed serialized component vocabulary; new cases change the link contract
pub enum TextNbtSource {
    /// Block position expression.
    Block(String),
    /// Entity selector expression.
    Entity(String),
    /// Command storage identifier.
    Storage(String),
}

/// An inline object embedded in text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// exhaustive: closed serialized component vocabulary; new cases change the link contract
pub enum TextObject {
    /// A player head, retaining the profile properties carried by the component.
    Player {
        /// Profile data, including signed skin properties when present.
        profile: serde_json::Value,
        /// Whether to draw the hat layer.
        hat: bool,
        /// Optional component for consumers unable to display the object; None uses the default.
        fallback: Option<Box<Text>>,
    },
    /// A sprite in a named atlas.
    Atlas {
        /// Atlas identifier.
        atlas: String,
        /// Sprite identifier.
        sprite: String,
        /// Optional component for consumers unable to display the object; None uses the default.
        fallback: Option<Box<Text>>,
    },
}

/// Optional style fields inherit; explicit `false` overrides an inherited decoration.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TextStyle {
    /// Named colour or `#RRGGBB` component colour.
    pub color: Option<String>,
    /// Packed ARGB shadow colour; zero suppresses shadows.
    pub shadow_color: Option<u32>,
    /// Bold decoration.
    pub bold: Option<bool>,
    /// Italic decoration.
    pub italic: Option<bool>,
    /// Underline decoration.
    pub underlined: Option<bool>,
    /// Strikethrough decoration.
    pub strikethrough: Option<bool>,
    /// Obfuscated decoration.
    pub obfuscated: Option<bool>,
    /// Action invoked by clicking the component.
    pub click: Option<TextClick>,
    /// Information displayed on hover.
    pub hover: Option<TextHover>,
    /// Text inserted into the chat input on Shift-click.
    pub insertion: Option<String>,
    /// Font identifier.
    pub font: Option<String>,
    /// Unknown style metadata remains owned data, without engine types.
    #[serde(default)]
    pub metadata: std::collections::BTreeMap<String, serde_json::Value>,
}

/// A text component's click action.
///
/// File-opening actions are intentionally absent: mods can neither produce nor
/// round-trip `OpenFile`. Capture drops untrusted file-opening action metadata.
/// Part of the [per-node origin tracking follow-up](https://github.com/arismoko/horizon/issues/1738).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// exhaustive: closed serialized component vocabulary; new cases change the link contract
pub enum TextClick {
    /// Open an HTTP or HTTPS URL.
    OpenUrl(String),
    /// Switch to the numbered page of a book.
    ChangePage(i32),
    /// Put a command in the chat input.
    SuggestCommand(String),
    /// Run a command.
    RunCommand(String),
    /// Copy text to the clipboard.
    CopyToClipboard(String),
    /// Show a dialog identifier or inline declaration.
    ShowDialog(serde_json::Value),
    /// A server-defined action with optional payload.
    Custom {
        /// Action identifier.
        id: String,
        /// Payload retained without interpretation.
        payload: Option<serde_json::Value>,
    },
}

/// A text component's hover action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// exhaustive: closed serialized component vocabulary; new cases change the link contract
pub enum TextHover {
    /// Show another styled component tree.
    Text(Box<Text>),
    /// Show an item with its component patch.
    Item {
        /// Item identifier, not a process-local registry slot.
        id: String,
        /// Display count.
        count: u32,
        /// Component patch carried by the component, if any.
        components: Option<serde_json::Value>,
    },
    /// Show an entity's identity and optional name.
    Entity {
        /// Entity type identifier.
        kind: String,
        /// UUID as the high and low halves.
        id: [u64; 2],
        /// Optional styled entity name.
        name: Option<Box<Text>>,
    },
}

impl Default for Text {
    fn default() -> Self {
        Self::literal("")
    }
}
impl Text {
    /// A component with the given contents, with no style overrides or children.
    pub fn new(content: TextContent) -> Self {
        Self {
            content,
            style: TextStyle::default(),
            children: Vec::new(),
            metadata: Default::default(),
        }
    }
    /// Check vanilla's component-depth bound before cloning any node payload.
    /// Counts every recursive component relation, including hover names.
    pub fn validate_depth(&self) -> Result<(), TextFormError> {
        let mut pending = vec![(self, 1)];
        while let Some((text, depth)) = pending.pop() {
            if depth > MAX_TEXT_DEPTH {
                return Err(TextFormError::TooDeep);
            }
            pending.extend(text.children.iter().map(|text| (text, depth + 1)));
            match &text.content {
                TextContent::Translatable { arguments, .. } => {
                    pending.extend(arguments.iter().map(|text| (text, depth + 1)))
                }
                TextContent::Selector { separator, .. } | TextContent::Nbt { separator, .. } => {
                    pending.extend(separator.iter().map(|text| (text.as_ref(), depth + 1)))
                }
                TextContent::Object(
                    TextObject::Player { fallback, .. } | TextObject::Atlas { fallback, .. },
                ) => pending.extend(fallback.iter().map(|text| (text.as_ref(), depth + 1))),
                _ => {}
            }
            match &text.style.hover {
                Some(TextHover::Text(text)) => pending.push((text, depth + 1)),
                Some(TextHover::Entity { name, .. }) => {
                    pending.extend(name.iter().map(|text| (text.as_ref(), depth + 1)))
                }
                _ => {}
            }
        }
        Ok(())
    }
    /// A literal component, with no inherited style overrides or children.
    pub fn literal(value: impl Into<String>) -> Self {
        Self::new(TextContent::Literal(value.into()))
    }
    /// A translatable component. The key and arguments remain unresolved data.
    pub fn translate(key: impl Into<String>, arguments: Vec<Text>) -> Self {
        Self::new(TextContent::Translatable {
            key: key.into(),
            fallback: None,
            arguments,
        })
    }
    /// Append a child without flattening its style or translation.
    pub fn append(mut self, child: Text) -> Self {
        self.children.push(child);
        self
    }
    /// Set this component's style; children continue to inherit unset fields.
    pub fn with_style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }
}

/// Flat representation of a component tree.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextForm {
    /// Nodes in parent-before-child order.
    pub nodes: Vec<TextNode>,
}
/// One shallow component and its position in the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextNode {
    /// Parent node index, or None for the root.
    pub parent: Option<u32>,
    /// Relation to the parent (child, argument, separator, fallback or hover).
    pub relation: TextRelation,
    /// Serialized shallow component, with recursive fields removed.
    pub data: String,
}
/// Node relation. Case order is append-only because it is part of the wasm link contract.
/// Unknown discriminants are rejected by generated bindings before `Text::try_from`; inconsistent tables return `Malformed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextRelation {
    /// Appended component.
    Child,
    /// Translation argument.
    Argument,
    /// Selector or NBT separator.
    Separator,
    /// Inline object fallback.
    Fallback,
    /// Hover text.
    HoverText,
    /// Hover entity name.
    HoverEntityName,
}
/// Maximum depth of a component tree, counted in nodes.
pub const MAX_TEXT_DEPTH: usize = 512;
/// Maximum nesting of JSON containers within an individual node.
pub const MAX_TEXT_FORM_DEPTH: usize = 100;

/// A text form exceeds a boundary or is malformed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextFormError {
    /// The host has no game backend for this operation.
    Unavailable,
    /// The component or its metadata exceeds the permitted depth.
    TooDeep,
    /// A node is not valid serialized data.
    Invalid(String),
    /// Parent indices or node relations are invalid.
    Malformed,
}
impl std::fmt::Display for TextFormError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => write!(f, "text backend unavailable"),
            Self::TooDeep => write!(f, "text form exceeds maximum depth"),
            Self::Invalid(error) => f.write_str(error),
            Self::Malformed => write!(f, "malformed text node table"),
        }
    }
}
impl std::error::Error for TextFormError {}
impl crate::NoBackend for TextFormError {
    fn no_backend() -> Self {
        Self::Unavailable
    }
}

// Bound the residual JSON metadata independently of component depth.
#[derive(Default)]
struct DepthWriter {
    bytes: Vec<u8>,
    depth: usize,
    quoted: bool,
    escaped: bool,
    too_deep: bool,
}
impl std::io::Write for DepthWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        for &byte in bytes {
            if self.quoted {
                if self.escaped {
                    self.escaped = false;
                } else if byte == b'\\' {
                    self.escaped = true;
                } else if byte == b'"' {
                    self.quoted = false;
                }
            } else {
                match byte {
                    b'"' => self.quoted = true,
                    b'{' | b'[' => {
                        self.depth += 1;
                        if self.depth > MAX_TEXT_FORM_DEPTH {
                            self.too_deep = true;
                            return Err(std::io::Error::other("text form too deep"));
                        }
                    }
                    b'}' | b']' => self.depth = self.depth.saturating_sub(1),
                    _ => {}
                }
            }
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl TryFrom<&Text> for TextForm {
    type Error = TextFormError;
    fn try_from(value: &Text) -> Result<Self, Self::Error> {
        value.validate_depth()?;
        let mut nodes = Vec::new();
        let mut pending = vec![(value, None, TextRelation::Child, 1)];
        while let Some((text, parent, relation, depth)) = pending.pop() {
            let index = u32::try_from(nodes.len()).map_err(|_| TextFormError::Malformed)?;
            let mut children = Vec::new();
            for child in &text.children {
                children.push((child, TextRelation::Child));
            }
            let content = match &text.content {
                TextContent::Literal(s) => TextContent::Literal(s.clone()),
                TextContent::Keybind(s) => TextContent::Keybind(s.clone()),
                TextContent::Score { name, objective } => TextContent::Score {
                    name: name.clone(),
                    objective: objective.clone(),
                },
                TextContent::Translatable {
                    key,
                    fallback,
                    arguments,
                } => {
                    children.extend(arguments.iter().map(|x| (x, TextRelation::Argument)));
                    TextContent::Translatable {
                        key: key.clone(),
                        fallback: fallback.clone(),
                        arguments: Vec::new(),
                    }
                }
                TextContent::Selector { pattern, separator } => {
                    if let Some(x) = separator {
                        children.push((x.as_ref(), TextRelation::Separator));
                    }
                    TextContent::Selector {
                        pattern: pattern.clone(),
                        separator: None,
                    }
                }
                TextContent::Nbt {
                    path,
                    interpret,
                    source,
                    separator,
                } => {
                    if let Some(x) = separator {
                        children.push((x.as_ref(), TextRelation::Separator));
                    }
                    TextContent::Nbt {
                        path: path.clone(),
                        interpret: *interpret,
                        source: source.clone(),
                        separator: None,
                    }
                }
                TextContent::Object(TextObject::Player {
                    profile,
                    hat,
                    fallback,
                }) => {
                    if let Some(x) = fallback {
                        children.push((x.as_ref(), TextRelation::Fallback));
                    }
                    TextContent::Object(TextObject::Player {
                        profile: profile.clone(),
                        hat: *hat,
                        fallback: None,
                    })
                }
                TextContent::Object(TextObject::Atlas {
                    atlas,
                    sprite,
                    fallback,
                }) => {
                    if let Some(x) = fallback {
                        children.push((x.as_ref(), TextRelation::Fallback));
                    }
                    TextContent::Object(TextObject::Atlas {
                        atlas: atlas.clone(),
                        sprite: sprite.clone(),
                        fallback: None,
                    })
                }
            };
            let hover = match &text.style.hover {
                Some(TextHover::Text(x)) => {
                    children.push((x.as_ref(), TextRelation::HoverText));
                    None
                }
                Some(TextHover::Entity { kind, id, name }) => {
                    if let Some(x) = name {
                        children.push((x.as_ref(), TextRelation::HoverEntityName));
                    }
                    Some(TextHover::Entity {
                        kind: kind.clone(),
                        id: *id,
                        name: None,
                    })
                }
                other => other.clone(),
            };
            let style = TextStyle {
                hover,
                ..text.style.clone_without_hover()
            };
            let shallow = Text {
                content,
                style,
                children: Vec::new(),
                metadata: text.metadata.clone(),
            };
            let mut writer = DepthWriter::default();
            if let Err(e) = serde_json::to_writer(&mut writer, &shallow) {
                return Err(if writer.too_deep {
                    TextFormError::TooDeep
                } else {
                    TextFormError::Invalid(e.to_string())
                });
            }
            nodes.push(TextNode {
                parent,
                relation,
                data: String::from_utf8(writer.bytes).expect("JSON is UTF-8"),
            });
            for (child, relation) in children.into_iter().rev() {
                pending.push((child, Some(index), relation, depth + 1));
            }
        }
        Ok(Self { nodes })
    }
}
impl TextStyle {
    // Listed deliberately: new fields must be handled; cloning the style would recurse into hover.
    fn clone_without_hover(&self) -> Self {
        Self {
            color: self.color.clone(),
            shadow_color: self.shadow_color,
            bold: self.bold,
            italic: self.italic,
            underlined: self.underlined,
            strikethrough: self.strikethrough,
            obfuscated: self.obfuscated,
            click: self.click.clone(),
            hover: None,
            insertion: self.insertion.clone(),
            font: self.font.clone(),
            metadata: self.metadata.clone(),
        }
    }
}
impl TryFrom<TextForm> for Text {
    type Error = TextFormError;
    fn try_from(value: TextForm) -> Result<Self, Self::Error> {
        use std::io::Write;
        let mut depths = Vec::with_capacity(value.nodes.len());
        for (index, node) in value.nodes.iter().enumerate() {
            let depth = match node.parent {
                None if index == 0 && node.relation == TextRelation::Child => 1,
                Some(parent) if (parent as usize) < index => depths[parent as usize] + 1,
                _ => return Err(TextFormError::Malformed),
            };
            if depth > MAX_TEXT_DEPTH {
                return Err(TextFormError::TooDeep);
            }
            depths.push(depth);
        }
        let mut nodes = Vec::with_capacity(value.nodes.len());
        for node in value.nodes {
            let mut writer = DepthWriter::default();
            writer
                .write_all(node.data.as_bytes())
                .map_err(|_| TextFormError::TooDeep)?;
            let text: Text = serde_json::from_str(&node.data)
                .map_err(|e: serde_json::Error| TextFormError::Invalid(e.to_string()))?;
            // Nodes must themselves be shallow; reject hidden recursive data.
            if !text.children.is_empty()
                || matches!(&text.content, TextContent::Translatable { arguments, .. } if !arguments.is_empty())
                || matches!(
                    &text.content,
                    TextContent::Selector {
                        separator: Some(_),
                        ..
                    } | TextContent::Nbt {
                        separator: Some(_),
                        ..
                    } | TextContent::Object(
                        TextObject::Player {
                            fallback: Some(_),
                            ..
                        } | TextObject::Atlas {
                            fallback: Some(_),
                            ..
                        }
                    )
                )
                || matches!(
                    &text.style.hover,
                    Some(TextHover::Text(_)) | Some(TextHover::Entity { name: Some(_), .. })
                )
            {
                return Err(TextFormError::Malformed);
            }
            nodes.push((node.parent, node.relation, Some(text)));
        }
        if nodes.is_empty() {
            return Err(TextFormError::Malformed);
        }
        for i in (1..nodes.len()).rev() {
            let parent = nodes[i].0.ok_or(TextFormError::Malformed)? as usize;
            let relation = nodes[i].1;
            let mut child = nodes[i].2.take().ok_or(TextFormError::Malformed)?;
            child.children.reverse();
            if let TextContent::Translatable { arguments, .. } = &mut child.content {
                arguments.reverse();
            }
            let target = nodes[parent].2.as_mut().ok_or(TextFormError::Malformed)?;
            match relation {
                TextRelation::Child => target.children.push(child),
                TextRelation::Argument => {
                    if let TextContent::Translatable { arguments, .. } = &mut target.content {
                        arguments.push(child);
                    } else {
                        return Err(TextFormError::Malformed);
                    }
                }
                TextRelation::Separator => match &mut target.content {
                    TextContent::Selector { separator, .. }
                    | TextContent::Nbt { separator, .. }
                        if separator.is_none() =>
                    {
                        *separator = Some(Box::new(child))
                    }
                    _ => return Err(TextFormError::Malformed),
                },
                TextRelation::Fallback => match &mut target.content {
                    TextContent::Object(
                        TextObject::Player { fallback, .. } | TextObject::Atlas { fallback, .. },
                    ) if fallback.is_none() => *fallback = Some(Box::new(child)),
                    _ => return Err(TextFormError::Malformed),
                },
                TextRelation::HoverText if target.style.hover.is_none() => {
                    target.style.hover = Some(TextHover::Text(Box::new(child)))
                }
                TextRelation::HoverEntityName => match &mut target.style.hover {
                    Some(TextHover::Entity { name, .. }) if name.is_none() => {
                        *name = Some(Box::new(child))
                    }
                    _ => return Err(TextFormError::Malformed),
                },
                _ => return Err(TextFormError::Malformed),
            }
        }
        let root = nodes[0].2.as_mut().ok_or(TextFormError::Malformed)?;
        root.children.reverse();
        if let TextContent::Translatable { arguments, .. } = &mut root.content {
            arguments.reverse();
        }
        nodes[0].2.take().ok_or(TextFormError::Malformed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deep_form_round_trip_on_small_stack() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut tree = Text::literal("leaf");
                for _ in 1..512 {
                    tree = Text::literal("parent").append(tree);
                }
                let form = TextForm::try_from(&tree).unwrap();
                let decoded = Text::try_from(form.clone()).unwrap();
                assert_eq!(TextForm::try_from(&decoded).unwrap(), form);
                let too_deep = Text::literal("root").append(tree);
                assert!(matches!(
                    TextForm::try_from(&too_deep),
                    Err(TextFormError::TooDeep)
                ));
                let mut over = form;
                let mut last = over.nodes.last().unwrap().clone();
                last.parent = Some(511);
                over.nodes.push(last);
                assert!(matches!(Text::try_from(over), Err(TextFormError::TooDeep)));
                std::mem::forget(too_deep);
                std::mem::forget(decoded);
            })
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn recursive_fields_and_order_survive() {
        let mut root =
            Text::translate("key", vec![Text::literal("first"), Text::literal("second")]);
        let mut selector = Text::literal("");
        selector.content = TextContent::Selector {
            pattern: "@a".into(),
            separator: Some(Box::new(Text::literal("sep"))),
        };
        let mut nbt = Text::literal("");
        nbt.content = TextContent::Nbt {
            path: "x".into(),
            interpret: false,
            source: TextNbtSource::Block("~ ~ ~".into()),
            separator: Some(Box::new(Text::literal("nbt"))),
        };
        let mut player = Text::literal("");
        player.content = TextContent::Object(TextObject::Player {
            profile: serde_json::json!({"name":"a"}),
            hat: true,
            fallback: Some(Box::new(Text::literal("player"))),
        });
        let mut atlas = Text::literal("");
        atlas.content = TextContent::Object(TextObject::Atlas {
            atlas: "a".into(),
            sprite: "b".into(),
            fallback: Some(Box::new(Text::literal("atlas"))),
        });
        player.style.hover = Some(TextHover::Text(Box::new(Text::literal("hover"))));
        atlas.style.hover = Some(TextHover::Entity {
            kind: "test".into(),
            id: [0, 1],
            name: Some(Box::new(Text::literal("name"))),
        });
        root.children = vec![selector, nbt, player, atlas];
        let form = TextForm::try_from(&root).unwrap();
        assert_eq!(Text::try_from(form).unwrap(), root);
    }
    #[test]
    fn invalid_parents_and_relations_refuse() {
        let form =
            TextForm::try_from(&Text::literal("root").append(Text::literal("child"))).unwrap();
        let mut bad = form.clone();
        bad.nodes[0].parent = Some(0);
        assert!(matches!(Text::try_from(bad), Err(TextFormError::Malformed)));
        let mut bad = form.clone();
        bad.nodes[1].parent = Some(1);
        assert!(matches!(Text::try_from(bad), Err(TextFormError::Malformed)));
        for relation in [
            TextRelation::Argument,
            TextRelation::Separator,
            TextRelation::Fallback,
            TextRelation::HoverEntityName,
        ] {
            let mut bad = form.clone();
            bad.nodes[1].relation = relation;
            assert!(matches!(Text::try_from(bad), Err(TextFormError::Malformed)));
        }
        let mut valid = form.clone();
        valid.nodes[1].relation = TextRelation::HoverText;
        assert!(Text::try_from(valid).is_ok());
    }
    #[test]
    fn via_accepts_infallible_and_fallible_forms() {
        #[horizon_mod_api_macros::wasm(via = String)]
        struct Infallible;
        impl From<&Infallible> for String {
            fn from(_: &Infallible) -> Self {
                String::new()
            }
        }
        impl TryFrom<String> for Infallible {
            type Error = ();
            fn try_from(_: String) -> Result<Self, Self::Error> {
                Ok(Self)
            }
        }
        #[horizon_mod_api_macros::wasm(via = TextForm)]
        struct Fallible(Text);
        impl TryFrom<&Fallible> for TextForm {
            type Error = TextFormError;
            fn try_from(value: &Fallible) -> Result<Self, Self::Error> {
                Self::try_from(&value.0)
            }
        }
        impl TryFrom<TextForm> for Fallible {
            type Error = TextFormError;
            fn try_from(value: TextForm) -> Result<Self, Self::Error> {
                Text::try_from(value).map(Self)
            }
        }
    }
    #[test]
    fn form_retains_translation_children_and_explicit_style_overrides() {
        let text = Text::translate(
            "chat.type.text",
            vec![Text::literal("Alex"), Text::literal("Hi")],
        )
        .with_style(TextStyle {
            bold: Some(true),
            ..Default::default()
        })
        .append(Text::literal("!").with_style(TextStyle {
            bold: Some(false),
            ..Default::default()
        }));
        let decoded = Text::try_from(TextForm::try_from(&text).unwrap()).unwrap();
        assert_eq!(decoded, text);
        assert!(
            matches!(&decoded.content, TextContent::Translatable { key, .. } if key == "chat.type.text")
        );
        assert_eq!(decoded.children[0].style.bold, Some(false));
    }
    #[test]
    fn object_fallback_form_distinguishes_omitted_styled_and_translatable() {
        use serde_json::json;
        let styled = json!({"content":{"Literal":"styled"}, "style":{"bold":false}, "children":[]});
        let translated = json!({"content":{"Translatable":{"key":"test.head", "arguments":[]}}, "style":{}, "children":[]});
        for object in [
            json!({"Player":{"profile":{"name":"Test"}, "hat":true}}),
            json!({"Atlas":{"atlas":"minecraft:blocks", "sprite":"test:icon"}}),
        ] {
            for fallback in [None, Some(styled.clone()), Some(translated.clone())] {
                let mut object = object.clone();
                if let Some(fallback) = &fallback {
                    object.as_object_mut().unwrap().values_mut().next().unwrap()["fallback"] =
                        fallback.clone();
                }
                let tree =
                    json!({"content":{"Object":object}, "style":{}, "children":[]}).to_string();
                let text: Text = serde_json::from_str(&tree).unwrap();
                let TextContent::Object(object) = &text.content else {
                    panic!("object contents")
                };
                let captured = match object {
                    TextObject::Player { fallback, .. } | TextObject::Atlas { fallback, .. } => {
                        fallback
                    }
                };
                match &fallback {
                    None => assert!(captured.is_none()),
                    Some(value) if value == &styled => {
                        assert_eq!(captured.as_ref().unwrap().style.bold, Some(false))
                    }
                    Some(_) => assert!(
                        matches!(&captured.as_ref().unwrap().content, TextContent::Translatable { key, .. } if key == "test.head")
                    ),
                }
                assert_eq!(
                    Text::try_from(TextForm::try_from(&text).unwrap()).unwrap(),
                    text
                );
            }
        }
    }
    #[test]
    fn form_keeps_unknown_metadata_and_refuses_malformed_trees() {
        let mut value = serde_json::to_value(Text::literal("value")).unwrap();
        value["metadata"]["future_component"] = serde_json::json!({"version": 7});
        value["style"]["metadata"]["future_style"] = serde_json::json!([1, 2]);
        let text: Text = serde_json::from_value(value.clone()).unwrap();
        let decoded = Text::try_from(TextForm::try_from(&text).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), value);
        assert!(
            Text::try_from(TextForm {
                nodes: vec![TextNode {
                    parent: None,
                    relation: TextRelation::Child,
                    data: "{\"content\": 42}".into()
                }]
            })
            .is_err()
        );
    }
}
