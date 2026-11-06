//! Lossless source ranges for the one-shot Parts table move.
use quote::ToTokens;
use serde_json::{Value, json};
use std::io::Read;
use syn::{Item, spanned::Spanned};

fn items(input: Vec<Item>) -> Vec<Value> {
    input
        .into_iter()
        .map(|item| {
            let span = item.span();
            let mut value = json!({"start": span.start().line, "end": span.end().line});
            let members = match &item {
                Item::Trait(t) => Some(
                    t.items
                        .iter()
                        .map(|i| (i.span(), i.to_token_stream()))
                        .collect::<Vec<_>>(),
                ),
                Item::Impl(i) => Some(
                    i.items
                        .iter()
                        .map(|i| (i.span(), i.to_token_stream()))
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            };
            if let Some(members) = members {
                value["members"] = json!(
                    members
                        .into_iter()
                        .map(|(s, t)| {
                            let text = t.to_string();
                            json!({"start": s.start().line, "end": s.end().line, "text": text})
                        })
                        .collect::<Vec<_>>()
                );
            }
            if let Item::Mod(m) = &item
                && let Some((_, children)) = &m.content
            {
                value["children"] = json!(items(children.clone()));
            }
            value
        })
        .collect()
}

fn main() -> Result<(), String> {
    let mut source = String::new();
    std::io::stdin()
        .read_to_string(&mut source)
        .map_err(|e| e.to_string())?;
    let file = syn::parse_file(&source).map_err(|e| e.to_string())?;
    println!("{}", json!(items(file.items)));
    Ok(())
}
