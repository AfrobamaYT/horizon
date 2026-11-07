//! The internals catalog as the generator reads it, and the names its table gives their functions.
use crate::crossing::{Crossing, Rule, bare};
use crate::internals::Internal;
use quote::{ToTokens, format_ident};
use std::collections::BTreeMap;

pub(crate) fn name(path: &str) -> syn::Ident {
    format_ident!("{}", bare(path).replace("::", "__").replace("r#", ""))
}

fn field_name(field: &crate::crossing::Field<'_>) -> String {
    match field.case {
        Some((_, case)) => format!(
            "{}_{}",
            heck::AsSnakeCase(case.to_string()),
            field.member.to_token_stream()
        ),
        None => field.member.to_token_stream().to_string(),
    }
}

fn accessor_name(field: &str) -> String {
    if field.chars().all(|c| c.is_ascii_digit()) {
        format!("field_{field}")
    } else {
        field.into()
    }
}

/// Names depend on the complete catalog layout, never on a mod's selected fields.
pub(crate) fn method_name(rule: &Rule, owner: &str, func: &str) -> String {
    let named = rule.named(owner).unwrap();
    let reserved = |func: &str| {
        (func == "case" && matches!(named.item, syn::Item::Enum(_)))
            || rule.fields(owner).iter().any(|field| {
                let crossing = rule.crossing_of(&rule.shape(&named.module, Some(owner), field.ty));
                if crossing == Crossing::Opaque {
                    return false;
                }
                let field = field_name(field);
                let get = accessor_name(&field);
                let get = get.trim_start_matches("r#");
                func.trim_start_matches("r#") == get
                    || (matches!(crossing, Crossing::Copy(_))
                        && (func == format!("set_{get}")
                            || func == descriptor_name(&field, Some(owner), rule)))
            })
    };
    let mut name = func.to_owned();
    while reserved(&name) {
        name = format!("call_{}", name.trim_start_matches("r#"));
    }
    name
}

/// An owned catalog boundary. Static tags in `Internal` are selected here, never leaked strings.
pub fn catalog(text: &str) -> Result<Vec<Internal>, String> {
    #[derive(serde::Deserialize)]
    struct Entry {
        path: String,
        kind: String,
        signature: String,
        hash: String,
        #[serde(default)]
        cfg: String,
        #[serde(default)]
        shape_only: bool,
        #[serde(default)]
        opaque_reason: Option<String>,
        #[serde(default)]
        refs: BTreeMap<String, String>,
    }
    let entries: Vec<Entry> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    entries
        .into_iter()
        .map(|e| {
            Ok(Internal {
                path: e.path,
                kind: match e.kind.as_str() {
                    "struct" => "struct",
                    "enum" => "enum",
                    "field" => "field",
                    "method" => "method",
                    "fn" => "fn",
                    "const" => "const",
                    "static" => "static",
                    "type" => "type",
                    "trait" => "trait",
                    "mod" => "mod",
                    _ => return Err(format!("unknown internal kind {}", e.kind)),
                },
                signature: e.signature,
                hash: e.hash,
                cfg: e.cfg,
                shape_only: e.shape_only,
                opaque_reason: e.opaque_reason.map(|_| "opaque layout"),
                refs: e.refs,
            })
        })
        .collect()
}

fn descriptor_name(field: &str, owner: Option<&str>, rule: &Rule) -> String {
    let mut name = format!("{}_field", accessor_name(field).trim_start_matches("r#"));
    if let Some(owner) = owner {
        let fields = rule.fields(owner);
        while fields.iter().any(|f| {
            let get = accessor_name(&field_name(f));
            name == get.trim_start_matches("r#")
                || name == format!("set_{}", get.trim_start_matches("r#"))
        }) {
            name = format!("call_{name}");
        }
    }
    name
}
