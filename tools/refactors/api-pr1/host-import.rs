//! One-shot: writes the helpers the catalog declared and the game's `native_*` functions signed
//! as hand-owned files: `horizon-mod-api/src/host/<area>.rs` (the traits) and
//! `horizon-client/src/game/host/<area>.rs` (their forwarders), and the old generated host's
//! manifest. Usage: `host-import ROOT OUT_DIR [AREAS_TSV]` (`AREAS_TSV`: `helper<TAB>area`, the
//! table that regroups; the default area is the declaring file).
use heck::ToUpperCamelCase;
use horizon_mod_api_gen::{Imported, ImportedTrait, import_hosts};
use std::{collections::BTreeMap, fmt::Write as _, fs, path::PathBuf};

/// The trait a record joins in its area: the old trait name, with client reads split off.
fn suffix(record: &Imported) -> &'static str {
    match record.owner.as_str() {
        "Host" => "Host",
        "ServerHost" => "ServerHost",
        "ClientHost" => "ClientHost",
        "Backend" if record.client_read => "ClientBackend",
        "Backend" => "Backend",
        "BackendMut" => "BackendMut",
        "ClientBackendMut" => "ClientBackendMut",
        _ => "",
    }
}

/// Parse `text` as a file and print it the way rustfmt does.
fn format(text: &str) -> String {
    let file = syn::parse_file(text).unwrap_or_else(|e| panic!("{e}\n{text}"));
    prettyplease::unparse(&file)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    let table: BTreeMap<String, String> = args
        .get(3)
        .map(|path| {
            fs::read_to_string(path)
                .unwrap()
                .lines()
                .filter_map(|line| line.split_once('\t'))
                .map(|(a, b)| (a.to_owned(), b.to_owned()))
                .collect()
        })
        .unwrap_or_default();
    let (records, traits, old_host, _) = import_hosts(&root).unwrap();
    fs::create_dir_all(out.join("mod-api")).unwrap();
    fs::create_dir_all(out.join("game")).unwrap();
    fs::write(out.join("old-generated-host.rs"), &old_host).unwrap();
    let is_trait = |name: &str| traits.iter().find(|t| t.name == name);
    let draw: Vec<&ImportedTrait> = traits.iter().filter(|t| t.game_struct.is_some()).collect();
    // area -> suffix -> records, with the original order kept.
    let mut areas: BTreeMap<String, BTreeMap<String, Vec<&Imported>>> = BTreeMap::new();
    // service and draw traits: trait -> records
    let mut own: BTreeMap<String, Vec<&Imported>> = BTreeMap::new();
    for record in &records {
        if is_trait(&record.owner).is_some() {
            own.entry(record.owner.clone()).or_default().push(record);
            continue;
        }
        let area = table
            .get(&record.name)
            .cloned()
            .unwrap_or_else(|| record.stem.clone());
        areas
            .entry(area)
            .or_default()
            .entry(suffix(record).to_owned())
            .or_default()
            .push(record);
    }
    let mut summary = String::new();
    // Every file: the traits first (API) and their forwarders (game).
    let mut stems: Vec<String> = areas.keys().cloned().collect();
    stems.push("services".into());
    stems.push("draw".into());
    stems.sort();
    stems.dedup();
    for stem in &stems {
        let camel = stem.to_upper_camel_case();
        let mut api = String::new();
        let mut game = String::new();
        let mut count = 0usize;
        let by_suffix = areas.get(stem).cloned().unwrap_or_default();
        let mut tiers: Vec<(&str, &str)> = vec![
            ("Host", "The host's methods both sides share"),
            ("ServerHost", "The server host's methods"),
            ("ClientHost", "The client host's methods"),
            ("Backend", "The handles' reads"),
            ("ClientBackend", "The handles' client-side reads"),
            ("BackendMut", "The handles' server writes"),
            ("ClientBackendMut", "The handles' client writes"),
        ];
        if stem == "draw" || stem == "services" {
            tiers.retain(|(s, _)| *s == "Host" || *s == "ServerHost" || *s == "ClientHost");
        }
        for (suffix, what) in tiers {
            let Some(list) = by_suffix.get(suffix) else {
                continue;
            };
            let name = format!("{camel}{suffix}");
            let hidden = if suffix.contains("Backend") {
                "#[doc(hidden)]\n"
            } else {
                ""
            };
            let _ = writeln!(
                api,
                "/// {what} of the `{stem}` area, implemented by the game and reached through [`{}`].\n{hidden}pub trait {name} {{",
                match suffix {
                    "Host" => "Host",
                    "ServerHost" => "ServerHost",
                    "ClientHost" => "ClientHost",
                    _ => "Backend",
                }
            );
            let _ = writeln!(game, "impl {name} for Game<'_> {{");
            for record in list {
                count += 1;
                api.push_str(&declaration(record));
                game.push_str(&forwarder(record));
            }
            api.push_str("}\n");
            game.push_str("}\n");
        }
        if stem == "services" || stem == "draw" {
            let wanted: Vec<&ImportedTrait> = if stem == "draw" {
                draw.clone()
            } else {
                traits.iter().filter(|t| t.game_struct.is_none()).collect()
            };
            for t in wanted {
                let list = own.get(&t.name).cloned().unwrap_or_default();
                let _ = writeln!(api, "/// {}\npub trait {} {{", t.doc.replace('\n', " "), t.name);
                if let Some((game_struct, surface)) = &t.game_struct {
                    let _ = writeln!(
                        game,
                        "/// [`{}`] on the game's `{surface}`.\npub(crate) struct {game_struct}<'a, 'g> {{\npub(crate) game: &'a Game<'g>,\npub(crate) surface: &'a mut {surface},\n}}\nimpl {} for {game_struct}<'_, '_> {{",
                        t.name, t.name
                    );
                } else {
                    let _ = writeln!(game, "impl {} for Game<'_> {{", t.name);
                }
                for record in list {
                    count += 1;
                    api.push_str(&declaration(record));
                    game.push_str(&forwarder(record));
                }
                api.push_str("}\n");
                game.push_str("}\n");
            }
        }
        let imports = |code: &str| {
            let mut lines = String::new();
            if code.contains("PathBuf") {
                lines.push_str("use std::path::PathBuf;\n");
            }
            if code.contains("Arc<") || code.contains("Arc::") {
                lines.push_str("use std::sync::Arc;\n");
            }
            lines
        };
        let api_file = format!(
            "//! The `{stem}` area: host methods the game implements (`game/host/{stem}.rs`).\n#![allow(unused_variables)]\nuse crate::*;\n{}{api}",
            imports(&api)
        );
        let game_file = format!(
            "//! The game's side of the `{stem}` area (`horizon-mod-api/src/host/{stem}.rs`).\nuse crate::game::Game;\nuse mod_api::{{prelude::*, *}};\nuse mod_api::__private::game::*;\n{}{game}",
            imports(&game)
        );
        fs::write(out.join(format!("mod-api/{stem}.rs")), format(&api_file)).unwrap();
        fs::write(out.join(format!("game/{stem}.rs")), format(&game_file)).unwrap();
        let _ = writeln!(summary, "{stem}\t{count}");
    }
    fs::write(out.join("areas.tsv"), summary).unwrap();
}

/// The trait's declaration: the helper's docs and signature, and the default a test double
/// gets (its access check, then the panic naming it).
fn declaration(record: &Imported) -> String {
    let mut method = syn::parse_str::<syn::TraitItemFn>(&record.declaration)
        .unwrap_or_else(|e| panic!("{}: {e}\n{}", record.name, record.declaration));
    let name = &record.name;
    let owner = &record.owner;
    let message = format!("{owner}::{name}");
    method.default = Some(if record.access.is_empty() {
        syn::parse_quote!({ unimplemented!(#message) })
    } else {
        let access = syn::Ident::new(record.access, proc_macro2::Span::call_site());
        syn::parse_quote!({
            crate::phase::unprovided(#message, #name, crate::phase::LoadAccess::#access)
        })
    });
    quote::ToTokens::to_token_stream(&method).to_string() + "\n"
}

/// The game's forwarder: the access check, then the `native_*` call.
fn forwarder(record: &Imported) -> String {
    let mut method = syn::parse_str::<syn::ImplItemFn>(&record.implementation)
        .unwrap_or_else(|e| panic!("{}: {e}\n{}", record.name, record.implementation));
    if !record.access.is_empty() {
        let name = &record.name;
        let access = syn::Ident::new(record.access, proc_macro2::Span::call_site());
        method.block.stmts.insert(
            0,
            syn::parse_quote!(mod_api::__private::check_access(#name, mod_api::__private::LoadAccess::#access);),
        );
    }
    quote::ToTokens::to_token_stream(&method).to_string() + "\n"
}
