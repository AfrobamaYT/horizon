//! The staged internals view (#589). Source copies only: the working tree keeps its visibility.
use proc_macro2::Span;
use quote::{ToTokens, quote};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use syn::{Fields, ImplItem, Item, Visibility, spanned::Spanned};

/// One exposed signature, addressed as an injection target.
#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct Internal {
    pub path: String,
    pub kind: &'static str,
    pub signature: String,
    pub hash: String,
}

/// The result of widening a source copy, including the deterministic manifest.
#[derive(Debug, Default)]
pub struct Widened {
    pub entries: Vec<Internal>,
    /// Items whose visibility actually changed, grouped by kind.
    pub changed: BTreeMap<&'static str, usize>,
    pub files: usize,
}

struct Source {
    path: PathBuf,
    text: String,
    lines: Vec<usize>,
    parsed: syn::File,
    edits: BTreeMap<usize, (usize, &'static str)>,
}

impl Source {
    fn offset(&self, position: proc_macro2::LineColumn) -> usize {
        let line = self.lines[position.line - 1];
        self.text[line..]
            .char_indices()
            .nth(position.column)
            .map_or(self.text.len(), |(i, _)| line + i)
    }

    fn public(&mut self, vis: &Visibility, start: Span, kind: &'static str) {
        if matches!(vis, Visibility::Public(_)) {
            return;
        }
        let (start, end) = match vis {
            Visibility::Inherited => (self.offset(start.start()), self.offset(start.start())),
            _ => (
                self.offset(vis.span().start()),
                self.offset(vis.span().end()),
            ),
        };
        self.edits.insert(start, (end, kind));
    }
}

#[derive(Default)]
struct Pass {
    sources: Vec<Source>,
    entries: BTreeMap<String, Internal>,
}

// Only test-only declarations are excluded. Other cfgs keep their spelling and visibility;
// changing visibility doesn't select an OS or feature for the staged build.
fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && matches!(
                attr.parse_args::<syn::Meta>(), Ok(syn::Meta::Path(path)) if path.is_ident("test")
            )
    })
}

fn clean_fields(fields: &Fields) -> Fields {
    let mut fields = fields.clone();
    for field in &mut fields {
        field.attrs.clear();
        field.vis = Visibility::Inherited;
    }
    fields
}

fn owner(ty: &syn::Type) -> String {
    struct EraseLifetimes;
    impl syn::visit_mut::VisitMut for EraseLifetimes {
        fn visit_lifetime_mut(&mut self, lifetime: &mut syn::Lifetime) {
            *lifetime = syn::Lifetime::new("'_", lifetime.span());
        }
    }
    let mut ty = ty.clone();
    syn::visit_mut::VisitMut::visit_type_mut(&mut EraseLifetimes, &mut ty);
    ty.to_token_stream().to_string().replace(' ', "")
}

fn type_path(module: &str, ident: &syn::Ident, generics: &syn::Generics) -> String {
    let (_, arguments, _) = generics.split_for_impl();
    let ty: syn::Type = syn::parse_quote!(#ident #arguments);
    format!("{module}::{}", owner(&ty))
}

impl Pass {
    fn load(&mut self, path: &Path) -> Result<usize, String> {
        if let Some(index) = self.sources.iter().position(|source| source.path == path) {
            return Ok(index);
        }
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let parsed = syn::parse_file(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let lines = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        let index = self.sources.len();
        self.sources.push(Source {
            path: path.to_owned(),
            text,
            parsed,
            lines,
            edits: BTreeMap::new(),
        });
        Ok(index)
    }

    fn entry(&mut self, path: &str, kind: &'static str, tokens: impl ToTokens) {
        let signature = tokens.to_token_stream().to_string();
        let digest = Sha256::digest(signature.as_bytes());
        let hash = digest[..8]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        self.entries.insert(
            path.to_owned(),
            Internal {
                path: path.to_owned(),
                kind,
                signature,
                hash,
            },
        );
    }

    fn fields(&mut self, file: usize, path: &str, fields: &Fields, widen: bool) {
        for (index, field) in fields.iter().enumerate() {
            if test_only(&field.attrs) {
                continue;
            }
            let name = field
                .ident
                .as_ref()
                .map_or_else(|| index.to_string(), ToString::to_string);
            if widen {
                let start = field
                    .ident
                    .as_ref()
                    .map_or_else(|| field.ty.span(), |ident| ident.span());
                self.sources[file].public(&field.vis, start, "field");
            }
            self.entry(&format!("{path}::{name}"), "field", &field.ty);
        }
    }

    fn member(&mut self, file: usize, path: &str, item: &ImplItem) {
        match item {
            ImplItem::Fn(method) if !test_only(&method.attrs) => {
                self.sources[file].public(&method.vis, method.sig.span(), "method");
                self.entry(
                    &format!("{path}::{}", method.sig.ident),
                    "method",
                    &method.sig,
                );
            }
            ImplItem::Const(value) if !test_only(&value.attrs) => {
                self.sources[file].public(&value.vis, value.const_token.span(), "const");
                let (ident, generics, ty) = (&value.ident, &value.generics, &value.ty);
                self.entry(
                    &format!("{path}::{ident}"),
                    "const",
                    quote!(const #ident #generics: #ty),
                );
            }
            ImplItem::Type(value) if !test_only(&value.attrs) => {
                self.sources[file].public(&value.vis, value.type_token.span(), "type");
                let (ident, generics, ty) = (&value.ident, &value.generics, &value.ty);
                self.entry(
                    &format!("{path}::{ident}"),
                    "type",
                    quote!(type #ident #generics = #ty),
                );
            }
            _ => {}
        }
    }

    // Does not visit expressions or blocks: local items, macros, uses and trait impls stay intact.
    fn items(&mut self, file: usize, module: &str, items: &[Item]) {
        for item in items {
            self.item(file, module, item, true);
        }
    }

    fn item(&mut self, file: usize, module: &str, item: &Item, children: bool) {
        macro_rules! expose {
            ($value:expr, $kind:literal, $start:expr, $signature:expr) => {{
                let value = $value;
                if test_only(&value.attrs) {
                    return;
                }
                let path = format!("{module}::{}", value.ident);
                self.sources[file].public(&value.vis, $start, $kind);
                self.entry(&path, $kind, $signature);
                path
            }};
        }
        match item {
            Item::Fn(value) if !test_only(&value.attrs) => {
                self.sources[file].public(&value.vis, value.sig.span(), "fn");
                self.entry(&format!("{module}::{}", value.sig.ident), "fn", &value.sig);
            }
            Item::Struct(value) => {
                let (ident, generics, fields) =
                    (&value.ident, &value.generics, clean_fields(&value.fields));
                expose!(
                    value,
                    "struct",
                    value.struct_token.span(),
                    quote!(struct #ident #generics #fields)
                );
                if children {
                    self.fields(
                        file,
                        &type_path(module, ident, generics),
                        &value.fields,
                        true,
                    );
                }
            }
            Item::Union(value) => {
                let (ident, generics) = (&value.ident, &value.generics);
                let fields = clean_fields(&Fields::Named(value.fields.clone()));
                expose!(
                    value,
                    "struct",
                    value.union_token.span(),
                    quote!(union #ident #generics #fields)
                );
                if children {
                    self.fields(
                        file,
                        &type_path(module, ident, generics),
                        &Fields::Named(value.fields.clone()),
                        true,
                    );
                }
            }
            Item::Enum(value) => {
                let (ident, generics) = (&value.ident, &value.generics);
                let mut variants = value.variants.clone();
                for variant in &mut variants {
                    variant.attrs.clear();
                    variant.fields = clean_fields(&variant.fields);
                }
                let path = expose!(
                    value,
                    "enum",
                    value.enum_token.span(),
                    quote!(enum #ident #generics { #variants })
                );
                if children {
                    for variant in &value.variants {
                        if !test_only(&variant.attrs) {
                            self.fields(
                                file,
                                &format!("{path}::{}", variant.ident),
                                &variant.fields,
                                false,
                            );
                        }
                    }
                }
            }
            Item::Const(value) => {
                let (ident, generics, ty) = (&value.ident, &value.generics, &value.ty);
                expose!(
                    value,
                    "const",
                    value.const_token.span(),
                    quote!(const #ident #generics: #ty)
                );
            }
            Item::Static(value) => {
                let (ident, mutability, ty) = (&value.ident, &value.mutability, &value.ty);
                expose!(
                    value,
                    "static",
                    value.static_token.span(),
                    quote!(static #mutability #ident: #ty)
                );
            }
            Item::Type(value) => {
                let (ident, generics, ty) = (&value.ident, &value.generics, &value.ty);
                expose!(
                    value,
                    "type",
                    value.type_token.span(),
                    quote!(type #ident #generics = #ty)
                );
            }
            Item::Trait(value) => {
                let mut signature = value.clone();
                signature.attrs.clear();
                signature.vis = Visibility::Inherited;
                // Trait method defaults are implementations, not part of the signature.
                for member in &mut signature.items {
                    if let syn::TraitItem::Fn(method) = member {
                        method.default = None;
                        method.semi_token = Some(Default::default());
                    }
                }
                expose!(value, "trait", value.trait_token.span(), signature);
            }
            Item::Mod(value) => {
                let ident = &value.ident;
                let path = expose!(value, "mod", value.mod_token.span(), quote!(mod #ident));
                if children && let Some((_, items)) = &value.content {
                    self.items(file, &path, items);
                }
            }
            Item::Impl(value) if children && value.trait_.is_none() && !test_only(&value.attrs) => {
                let path = format!("{module}::{}", owner(&value.self_ty));
                for member in &value.items {
                    self.member(file, &path, member);
                }
            }
            _ => {}
        }
    }

    fn lower(&mut self, root: &Path, path: &str) -> Result<(), String> {
        let parsed: syn::Path = syn::parse_str(path).map_err(|e| format!("invalid path: {e}"))?;
        let mut segments = parsed.segments.iter();
        let crate_name = segments.next().ok_or("empty path")?.ident.to_string();
        let package = root.join("crates").join(crate_name.replace('_', "-"));
        let manifest = fs::read_to_string(package.join("Cargo.toml")).map_err(|e| e.to_string())?;
        let manifest: toml::Value = toml::from_str(&manifest).map_err(|e| e.to_string())?;
        let lib = manifest
            .get("lib")
            .and_then(|lib| lib.get("path"))
            .and_then(toml::Value::as_str)
            .unwrap_or("src/lib.rs");
        let mut file = self.load(&package.join(lib))?;
        let mut dir = self.sources[file]
            .path
            .parent()
            .expect("source parent")
            .to_owned();
        let mut items = self.sources[file].parsed.items.clone();
        let mut module = crate_name;
        let segments: Vec<_> = segments.collect();
        for (index, segment) in segments.iter().enumerate() {
            let ident = &segment.ident;
            if let Some(Item::Mod(child)) = items.iter().find(|item| matches!(item, Item::Mod(value) if value.ident == *ident && !test_only(&value.attrs))) {
                self.item(file, &module, &Item::Mod(child.clone()), false);
                module = format!("{module}::{ident}");
                if index + 1 == segments.len() { return Ok(()); }
                if let Some((_, inner)) = &child.content { items = inner.clone(); dir = dir.join(ident.to_string()); }
                else {
                    let explicit = child.attrs.iter().find_map(|attr| {
                        if attr.path().is_ident("path") && let syn::Meta::NameValue(value) = &attr.meta && let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(path), .. }) = &value.value { Some(path.value()) } else { None }
                    });
                    let child_path = if let Some(relative) = explicit {
                        self.sources[file].path.parent().expect("source parent").join(relative)
                    } else if dir.join(format!("{ident}.rs")).is_file() { dir.join(format!("{ident}.rs")) }
                    else { dir.join(ident.to_string()).join("mod.rs") };
                    dir = if child_path.file_name().is_some_and(|name| name == "mod.rs") { child_path.parent().expect("source parent").to_owned() } else { child_path.with_extension("") };
                    file = self.load(&child_path)?;
                    items = self.sources[file].parsed.items.clone();
                }
                continue;
            }
            let rest = &segments[index..];
            if rest.len() == 1 {
                let selected = items
                    .iter()
                    .find(|item| match item {
                        Item::Fn(value) => value.sig.ident == *ident && !test_only(&value.attrs),
                        Item::Struct(value) => value.ident == *ident && !test_only(&value.attrs),
                        Item::Enum(value) => value.ident == *ident && !test_only(&value.attrs),
                        Item::Union(value) => value.ident == *ident && !test_only(&value.attrs),
                        Item::Const(value) => value.ident == *ident && !test_only(&value.attrs),
                        Item::Static(value) => value.ident == *ident && !test_only(&value.attrs),
                        Item::Type(value) => value.ident == *ident && !test_only(&value.attrs),
                        Item::Trait(value) => value.ident == *ident && !test_only(&value.attrs),
                        _ => false,
                    })
                    .ok_or("item not found")?;
                self.item(file, &module, selected, false);
                return Ok(());
            }
            if rest.len() == 2 {
                let member_name = &rest[1].ident;
                let mut found = false;
                for item in &items {
                    if let Item::Impl(block) = item
                        && block.trait_.is_none()
                        && !test_only(&block.attrs)
                        && let syn::Type::Path(ty) = &*block.self_ty
                        && ty
                            .path
                            .segments
                            .last()
                            .is_some_and(|part| part.ident == *ident)
                    {
                        for member in &block.items {
                            let matches = match member {
                                ImplItem::Fn(value) => value.sig.ident == *member_name,
                                ImplItem::Const(value) => value.ident == *member_name,
                                ImplItem::Type(value) => value.ident == *member_name,
                                _ => false,
                            };
                            if matches {
                                self.member(
                                    file,
                                    &format!("{module}::{}", owner(&block.self_ty)),
                                    member,
                                );
                                found = true;
                            }
                        }
                    }
                }
                if found {
                    for item in &items {
                        if matches!(item, Item::Struct(value) if value.ident == *ident)
                            || matches!(item, Item::Enum(value) if value.ident == *ident)
                            || matches!(item, Item::Union(value) if value.ident == *ident)
                        {
                            self.item(file, &module, item, false);
                        }
                    }
                    return Ok(());
                }
            }
            return Err("item not found".into());
        }
        Err("path names a crate, not an item".into())
    }

    fn finish(self) -> Result<Widened, String> {
        let mut widened = Widened {
            entries: self.entries.into_values().collect(),
            ..Widened::default()
        };
        for mut source in self.sources {
            if source.edits.is_empty() {
                continue;
            }
            for (start, (end, kind)) in source.edits.into_iter().rev() {
                source
                    .text
                    .replace_range(start..end, if start == end { "pub " } else { "pub" });
                *widened.changed.entry(kind).or_default() += 1;
            }
            fs::write(&source.path, source.text)
                .map_err(|e| format!("{}: {e}", source.path.display()))?;
            widened.files += 1;
        }
        Ok(widened)
    }
}

/// Widen game internals and the declared lower-crate items in a staged source copy.
/// Fails before writing anything if a declaration in `internals.txt` cannot be resolved.
pub fn widen(root: &Path) -> Result<Widened, String> {
    let list = root.join("crates/horizon-mod-api-gen/internals.txt");
    let text = fs::read_to_string(&list).map_err(|e| format!("{}: {e}", list.display()))?;
    widen_with(root, &text)
}

fn widen_with(root: &Path, list: &str) -> Result<Widened, String> {
    let mut pass = Pass::default();
    let src = root.join("crates/horizon-client/src");
    for source in crate::points::game_files(&src)?
        .into_iter()
        .chain(std::iter::once(crate::points::GameFile {
            module: "horizon_client::entry".into(),
            path: src.join("entry.rs"),
        }))
    {
        let file = pass.load(&source.path)?;
        pass.items(
            file,
            &source.module,
            &pass.sources[file].parsed.items.clone(),
        );
    }
    for (line, declaration) in list.lines().enumerate() {
        let path = declaration.split('#').next().unwrap_or_default().trim();
        if !path.is_empty() {
            pass.lower(root, path)
                .map_err(|e| format!("internals.txt:{}: {path}: {e}", line + 1))?;
        }
    }
    pass.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new(game: &str) -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "internals-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let fixture = Self(root);
            fixture.put("crates/horizon-client/src/game.rs", game);
            fixture.put("crates/horizon-client/src/entry.rs", "fn start() {}\n");
            fixture
        }
        fn put(&self, path: &str, text: &str) {
            let path = self.0.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        fn game(&self) -> String {
            fs::read_to_string(self.0.join("crates/horizon-client/src/game.rs")).unwrap()
        }
        fn lower(&self) {
            self.put(
                "crates/lower/Cargo.toml",
                "[package]\nname = 'lower'\nversion = '0.1.0'\n",
            );
            self.put("crates/lower/src/lib.rs", "mod hidden;\n");
            self.put(
                "crates/lower/src/hidden.rs",
                "#[path = \"other.rs\"] mod inner;\nfn sibling() {}\n",
            );
            self.put(
                "crates/lower/src/other.rs",
                "struct Thing(u32); impl Thing { fn run(&self) {} fn unrelated(&self) {} }\n",
            );
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn widens_items_without_touching_uses_traits_macros_or_local_items() {
        let fixture = Fixture::new(
            r#"use std::fmt;
pub(crate) struct Game<'a> { world: u32, marker: &'a str }
struct Tuple(#[allow(dead_code)] u8, pub(crate) u16);
enum Kind { Named { value: u8 }, Tuple(u16) }
union Bits { byte: u8, word: u32 }
const LIMIT: u32 = 4;
static COUNT: u32 = 0;
type Alias = u32;
trait Trait { fn call(&self); }
impl<'a> Game<'a> {
    const SIZE: u32 = 4;
    #[inline] pub(super) async fn call(&self) { struct Local; }
    #[cfg(test)] fn test_method() {}
}
impl Trait for Game<'_> { fn call(&self) {} }
macro_rules! untouched { () => { struct MacroItem; } }
#[cfg(test)] mod tests { struct Private; }
#[cfg(test)] struct TestItem;
mod inline { struct Inner; }
// café: span columns after non-ASCII text are character-based.
const LABEL: &str = "é"; struct AfterUnicode;
"#,
        );
        let result = widen_with(&fixture.0, "").unwrap();
        let source = fixture.game();
        for expected in [
            "pub struct Game",
            "pub world: u32",
            "pub marker:",
            "pub struct Tuple(#[allow(dead_code)] pub u8, pub u16)",
            "pub enum Kind { Named { value: u8 }, Tuple(u16) }",
            "pub union Bits { pub byte: u8, pub word: u32 }",
            "pub const LIMIT",
            "pub static COUNT",
            "pub type Alias",
            "pub trait Trait",
            "pub const SIZE",
            "pub async fn call",
            "pub mod inline { pub struct Inner; }",
            "pub struct AfterUnicode;",
        ] {
            assert!(
                source.contains(expected),
                "missing {expected:?} in {source}"
            );
        }
        for unchanged in [
            "use std::fmt;",
            "struct Local;",
            "#[cfg(test)] fn test_method() {}",
            "impl Trait for Game<'_> { fn call(&self) {} }",
            "macro_rules! untouched { () => { struct MacroItem; } }",
            "#[cfg(test)] mod tests { struct Private; }",
            "#[cfg(test)] struct TestItem;",
        ] {
            assert!(source.contains(unchanged), "changed {unchanged:?}");
        }
        syn::parse_file(&source).unwrap();
        let field = result
            .entries
            .iter()
            .find(|entry| entry.path == "horizon_client::game::Game<'_>::world")
            .unwrap();
        assert_eq!(field.signature, "u32");
        assert_eq!(field.hash, "d7649d428b9ff33d");
        assert!(
            result
                .entries
                .iter()
                .any(|entry| entry.path == "horizon_client::game::Game<'_>::call")
        );
        assert!(
            !result
                .entries
                .iter()
                .any(|entry| entry.path.contains("TestItem") || entry.path.contains("Local"))
        );
    }

    #[test]
    fn selected_lower_method_and_its_modules_and_receiver_become_reachable() {
        let fixture = Fixture::new("struct Game;\n");
        fixture.lower();
        let result = widen_with(&fixture.0, "# list\nlower::hidden::inner::Thing::run\n").unwrap();
        let read = |path| fs::read_to_string(fixture.0.join(path)).unwrap();
        assert_eq!(read("crates/lower/src/lib.rs"), "pub mod hidden;\n");
        assert_eq!(
            read("crates/lower/src/hidden.rs"),
            "#[path = \"other.rs\"] pub mod inner;\nfn sibling() {}\n"
        );
        assert_eq!(
            read("crates/lower/src/other.rs"),
            "pub struct Thing(u32); impl Thing { pub fn run(&self) {} fn unrelated(&self) {} }\n"
        );
        assert!(
            result
                .entries
                .iter()
                .any(|entry| entry.path == "lower::hidden::inner::Thing::run")
        );
        assert!(
            !result
                .entries
                .iter()
                .any(|entry| entry.path.contains("unrelated") || entry.path.contains("sibling"))
        );
    }

    #[test]
    fn unresolved_line_is_an_error_and_does_not_write_sources() {
        let fixture = Fixture::new("struct Game;\n");
        fixture.lower();
        let error = widen_with(
            &fixture.0,
            "# comment\nlower::hidden::inner::Thing::missing\n",
        )
        .unwrap_err();
        assert!(
            error.contains("internals.txt:2: lower::hidden::inner::Thing::missing"),
            "{error}"
        );
        assert_eq!(fixture.game(), "struct Game;\n");
    }

    #[test]
    fn manifest_is_sorted_deterministic_and_independent_of_visibility() {
        let game = "pub struct Game;\nimpl Game { pub fn z(&self) {} fn a(&self) {} }\n";
        let one = Fixture::new(game);
        let two = Fixture::new(game);
        let first = widen_with(&one.0, "").unwrap();
        let second = widen_with(&two.0, "").unwrap();
        assert_eq!(
            serde_json::to_string(&first.entries).unwrap(),
            serde_json::to_string(&second.entries).unwrap()
        );
        assert!(
            first
                .entries
                .windows(2)
                .all(|pair| pair[0].path < pair[1].path)
        );
        let again = widen_with(&one.0, "").unwrap();
        assert_eq!(first.entries, again.entries);
        assert!(again.changed.is_empty());
    }
}
