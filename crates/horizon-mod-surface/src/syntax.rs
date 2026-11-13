//! What the parser reads of one source file for [`crate::uses`]: where structs are built and
//! matched, which paths are written, which impl each `Self` stands in, what the file defines,
//! and the byte ranges of macro bodies, whose tokens the parser does not see into.
use crate::scan::Token;
use std::collections::BTreeSet;
use std::ops::Range;
use syn::visit::{self, Visit};

/// How a struct is built or matched at one place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum How {
    /// A literal or pattern that lists every field: `S { a, b }`, `S(a, b)`.
    Full,
    /// A literal that ends in `..` or a base: `S { a, ..d }`.
    Base,
    /// A name used as a value or bound in a pattern by itself (`.map(Pos)`, `Marker`): a build
    /// only for a tuple or unit struct.
    Value,
}

/// A place a struct may be built: the path written, as names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Build<'t> {
    /// The path's names; `Self` for `Self { … }`.
    pub path: Vec<&'t str>,
    /// Written with a leading `::`.
    pub rooted: bool,
    pub how: How,
    /// A brace literal (`S { … }`), for the `..Default::default()` advice.
    pub brace: bool,
    pub line: usize,
    /// The byte where the path starts.
    pub byte: usize,
}

/// An `impl` block: the bytes of its body, and the path of the type it is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Impl<'t> {
    pub body: Range<usize>,
    /// Its names, an empty first one for a leading `::`.
    pub target: Vec<&'t str>,
}

/// What the parser reads of one file.
#[derive(Debug, Default)]
pub(crate) struct Syntax<'t> {
    pub builds: Vec<Build<'t>>,
    /// Every path written in code, attributes included, `use` trees left out: its names.
    pub paths: Vec<Vec<&'t str>>,
    pub impls: Vec<Impl<'t>>,
    /// The structs, enums, unions and type aliases the file defines at its top level, under no
    /// `#[cfg]` or `#[cfg_attr]`.
    pub defines: BTreeSet<&'t str>,
    /// Every struct, enum and union defined anywhere, and every enum variant.
    pub types: BTreeSet<&'t str>,
    /// Every trait defined anywhere.
    pub traits: BTreeSet<&'t str>,
    /// `type X = …Y<…>;` anywhere: (X, Y), and whether it is an item of its own (not an
    /// associated type of an impl or trait).
    pub type_aliases: Vec<(&'t str, &'t str, bool)>,
    /// The file holds an inline `mod name { … }`, so it is more than one scope.
    pub inline_modules: bool,
    /// The bytes inside each macro call's delimiters, and each `macro_rules!` body.
    pub macro_bodies: Vec<Range<usize>>,
}

/// Whether an item carries `#[cfg(…)]` or `#[cfg_attr(…)]`.
fn conditional(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
}

struct Reader<'t> {
    text: &'t str,
    out: Syntax<'t>,
}

impl<'t> Reader<'t> {
    /// An identifier's name as written in the text, `r#` left out.
    fn name(&self, ident: &syn::Ident) -> &'t str {
        let written = self.text.get(ident.span().byte_range()).unwrap_or("");
        written.strip_prefix("r#").unwrap_or(written)
    }

    fn names(&self, path: &syn::Path) -> Vec<&'t str> {
        path.segments.iter().map(|s| self.name(&s.ident)).collect()
    }

    fn alias(&mut self, alias: &syn::Ident, ty: &syn::Type, item: bool) {
        let alias = self.name(alias);
        let target = match ty {
            syn::Type::Path(t) => t.path.segments.last().map(|last| self.name(&last.ident)),
            _ => None,
        };
        // A target the scan cannot read is recorded as the alias itself: unknown, so unsure.
        self.out
            .type_aliases
            .push((alias, target.unwrap_or(alias), item));
    }

    fn build(&mut self, path: &syn::Path, how: How, brace: bool) {
        let span = match (&path.leading_colon, path.segments.first()) {
            (Some(colons), _) => colons.spans[0],
            (None, Some(first)) => first.ident.span(),
            (None, None) => return,
        };
        self.out.builds.push(Build {
            path: self.names(path),
            rooted: path.leading_colon.is_some(),
            how,
            brace,
            line: span.start().line,
            byte: span.byte_range().start,
        });
    }
}

impl<'a, 't> Visit<'a> for Reader<'t> {
    fn visit_expr_struct(&mut self, e: &'a syn::ExprStruct) {
        let how = if e.dot2_token.is_some() || e.rest.is_some() {
            How::Base
        } else {
            How::Full
        };
        self.build(&e.path, how, true);
        visit::visit_expr_struct(self, e);
    }

    fn visit_expr_call(&mut self, e: &'a syn::ExprCall) {
        if let syn::Expr::Path(p) = &*e.func {
            self.build(&p.path, How::Full, false);
        }
        visit::visit_expr_call(self, e);
    }

    fn visit_expr_path(&mut self, e: &'a syn::ExprPath) {
        // A value (and in a pattern, `Pat::Path`): a tuple struct's constructor, a unit struct.
        self.build(&e.path, How::Value, false);
        visit::visit_expr_path(self, e);
    }

    fn visit_pat_struct(&mut self, p: &'a syn::PatStruct) {
        // With `..` a pattern reads the fields it names; without, it lists them all.
        if p.rest.is_none() {
            self.build(&p.path, How::Full, false);
        }
        visit::visit_pat_struct(self, p);
    }

    fn visit_pat_tuple_struct(&mut self, p: &'a syn::PatTupleStruct) {
        if !p.elems.iter().any(|e| matches!(e, syn::Pat::Rest(_))) {
            self.build(&p.path, How::Full, false);
        }
        visit::visit_pat_tuple_struct(self, p);
    }

    fn visit_pat_ident(&mut self, p: &'a syn::PatIdent) {
        // `Marker` alone in a pattern parses as a binding; it may be a unit struct.
        if p.subpat.is_none() && p.by_ref.is_none() && p.mutability.is_none() {
            self.build(&syn::Path::from(p.ident.clone()), How::Value, false);
        }
        visit::visit_pat_ident(self, p);
    }

    fn visit_path(&mut self, p: &'a syn::Path) {
        let names = self.names(p);
        self.out.paths.push(names);
        visit::visit_path(self, p);
    }

    fn visit_item_impl(&mut self, i: &'a syn::ItemImpl) {
        if let syn::Type::Path(t) = &*i.self_ty {
            let mut target = self.names(&t.path);
            if t.path.leading_colon.is_some() {
                target.insert(0, "");
            }
            self.out.impls.push(Impl {
                body: i.brace_token.span.join().byte_range(),
                target,
            });
        }
        visit::visit_item_impl(self, i);
    }

    fn visit_item_mod(&mut self, m: &'a syn::ItemMod) {
        self.out.inline_modules |= m.content.is_some();
        visit::visit_item_mod(self, m);
    }

    fn visit_item_struct(&mut self, i: &'a syn::ItemStruct) {
        let name = self.name(&i.ident);
        self.out.types.insert(name);
        visit::visit_item_struct(self, i);
    }

    fn visit_item_enum(&mut self, i: &'a syn::ItemEnum) {
        let name = self.name(&i.ident);
        self.out.types.insert(name);
        for variant in &i.variants {
            let name = self.name(&variant.ident);
            self.out.types.insert(name);
        }
        visit::visit_item_enum(self, i);
    }

    fn visit_item_union(&mut self, i: &'a syn::ItemUnion) {
        let name = self.name(&i.ident);
        self.out.types.insert(name);
        visit::visit_item_union(self, i);
    }

    fn visit_item_trait(&mut self, i: &'a syn::ItemTrait) {
        let name = self.name(&i.ident);
        self.out.traits.insert(name);
        visit::visit_item_trait(self, i);
    }

    fn visit_item_type(&mut self, i: &'a syn::ItemType) {
        self.alias(&i.ident, &i.ty, true);
        visit::visit_item_type(self, i);
    }

    fn visit_impl_item_type(&mut self, i: &'a syn::ImplItemType) {
        self.alias(&i.ident, &i.ty, false);
        visit::visit_impl_item_type(self, i);
    }

    fn visit_macro(&mut self, m: &'a syn::Macro) {
        let span = match &m.delimiter {
            syn::MacroDelimiter::Paren(d) => d.span.join(),
            syn::MacroDelimiter::Brace(d) => d.span.join(),
            syn::MacroDelimiter::Bracket(d) => d.span.join(),
        };
        self.out.macro_bodies.push(span.byte_range());
        visit::visit_macro(self, m);
    }
}

/// The deepest the parser is let recurse, counted by [`too_deep`].
pub(crate) const MAX_NESTING: usize = 256;

/// Keywords after which a token starts an operand rather than continuing one.
pub(crate) const KEYWORDS: &[&str] = &[
    "as", "async", "box", "break", "const", "continue", "dyn", "else", "enum", "extern", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "static", "struct", "trait", "type", "unsafe", "use", "where", "while", "yield",
];

/// Keywords the parser reads what follows by recursion: `return return x`, `fn() -> fn()`,
/// `if … else if …`.
const RECURSIVE: &[&str] = &[
    "async", "box", "break", "dyn", "fn", "if", "impl", "let", "move", "ref", "return", "yield",
];

/// The first line where the parser would recurse deeper than [`MAX_NESTING`], counted along the
/// path of nesting: each open bracket or generic `<`, and within a statement (up to its `;`,
/// `,` or a `}` that ends it) each prefix operator, closure, assignment, `@` and
/// recursive keyword. A file within it parses well inside the parser's stack.
pub(crate) fn too_deep(tokens: &[(Token<'_>, usize)]) -> Option<usize> {
    let token = |at: usize| tokens.get(at).map(|(t, _)| *t);
    let punct = |at: usize, byte: u8| token(at) == Some(Token::Punct(byte));
    // Whether the token before `at` ends an operand, so an operator at `at` is binary.
    let operand_end = |at: usize| match at.checked_sub(1).and_then(token) {
        Some(Token::Literal(_) | Token::Punct(b')' | b']' | b'}' | b'?')) => true,
        Some(Token::Ident(name)) => !KEYWORDS.contains(&name),
        _ => false,
    };
    // Each open bracket (`true`) or generic `<` (`false`), with the depth before it.
    let mut frames: Vec<(usize, bool)> = Vec::new();
    let mut depth = 0usize;
    // Inside a closure's parameters, whose closing `|` is no new closure.
    let mut params = false;
    let mut skip = false;
    for (at, (current, line)) in tokens.iter().enumerate() {
        let statement =
            |frames: &[(usize, bool)]| frames.last().map_or(0, |(before, _)| before + 1);
        if skip {
            skip = false;
            continue;
        }
        match current {
            Token::Punct(b'(' | b'[' | b'{') => {
                frames.push((depth, true));
                depth += 1;
                params = false;
            }
            Token::Punct(b')' | b']' | b'}') => {
                while let Some((before, bracket)) = frames.pop() {
                    depth = before;
                    if bracket {
                        break;
                    }
                }
                // A block that ends its statement: the next one starts afresh.
                if punct(at, b'}')
                    && match token(at + 1) {
                        None | Some(Token::Punct(b'}' | b';' | b',' | b'#' | b')' | b']')) => true,
                        Some(Token::Ident(next)) => !matches!(next, "else" | "as"),
                        _ => false,
                    }
                {
                    while frames.last().is_some_and(|(_, bracket)| !bracket) {
                        frames.pop();
                    }
                    depth = statement(&frames);
                }
            }
            Token::Punct(b';') => {
                while frames.last().is_some_and(|(_, bracket)| !bracket) {
                    frames.pop();
                }
                depth = statement(&frames);
                params = false;
            }
            Token::Punct(b',') => depth = statement(&frames),
            Token::Punct(b'<') if !punct(at + 1, b'=') => {
                frames.push((depth, false));
                depth += 1;
            }
            // `->` closes no generic; what follows it recurses through what is counted.
            Token::Punct(b'>') if at >= 1 && punct(at - 1, b'-') => {}
            Token::Punct(b'>') if !punct(at + 1, b'=') && !(at >= 1 && punct(at - 1, b'=')) => {
                if let Some((before, false)) = frames.last().copied() {
                    frames.pop();
                    depth = before;
                }
            }
            Token::Punct(b'|') => {
                if params {
                    params = false;
                } else if operand_end(at) {
                    // A binary `|` or `||`.
                    skip = punct(at + 1, b'|');
                } else {
                    // A closure: `||` has no parameters.
                    depth += 1;
                    skip = punct(at + 1, b'|');
                    params = !skip;
                }
            }
            Token::Punct(b'=')
                if !punct(at + 1, b'=')
                    && !punct(at + 1, b'>')
                    && !(at >= 1
                        && matches!(
                            token(at - 1),
                            Some(Token::Punct(b'=' | b'!' | b'<' | b'>'))
                        )) =>
            {
                depth += 1;
            }
            Token::Punct(b'@') => depth += 1,
            Token::Punct(b'-' | b'!' | b'*' | b'&' | b'.') if !operand_end(at) => depth += 1,
            Token::Ident(name) if RECURSIVE.contains(name) => depth += 1,
            _ => {}
        }
        if depth > MAX_NESTING {
            return Some(*line);
        }
    }
    None
}

/// Parses `text` and reads it; the line of the parse error when it does not parse.
pub(crate) fn read(text: &str) -> Result<Syntax<'_>, usize> {
    // The parser reads nesting by recursion. [`too_deep`] bounds it; a stack of its own leaves
    // room for any file within that bound, wherever the scan is called from.
    #[cfg(not(target_family = "wasm"))]
    {
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn_scoped(scope, || parse(text))
                .map_or_else(|_| parse(text), |thread| thread.join().unwrap_or(Err(1)))
        })
    }
    #[cfg(target_family = "wasm")]
    parse(text)
}

fn parse(text: &str) -> Result<Syntax<'_>, usize> {
    // The parser skips a byte-order mark and a shebang line itself, which would shift every
    // byte it reports; blank them in a copy of the same length instead.
    let mut copy = text.to_owned();
    if copy.starts_with('\u{feff}') {
        copy.replace_range(..3, "   ");
    }
    if copy.starts_with("#!") && !copy.starts_with("#![") {
        let end = copy.find('\n').unwrap_or(copy.len());
        copy.replace_range(..end, &" ".repeat(end));
    }
    let parsed = syn::parse_str::<syn::File>(&copy);
    let result = match parsed {
        Ok(file) => {
            let mut reader = Reader {
                text,
                out: Syntax::default(),
            };
            for item in &file.items {
                let defined = match item {
                    syn::Item::Struct(i) if !conditional(&i.attrs) => Some(&i.ident),
                    syn::Item::Enum(i) if !conditional(&i.attrs) => Some(&i.ident),
                    syn::Item::Union(i) if !conditional(&i.attrs) => Some(&i.ident),
                    syn::Item::Type(i) if !conditional(&i.attrs) => Some(&i.ident),
                    _ => None,
                };
                if let Some(ident) = defined {
                    let name = reader.name(ident);
                    reader.out.defines.insert(name);
                }
            }
            reader.visit_file(&file);
            Ok(reader.out)
        }
        Err(error) => Err(error.span().start().line.max(1)),
    };
    // The parser keeps every text it read for its spans until told to forget them.
    proc_macro2::extra::invalidate_current_thread_spans();
    result
}
