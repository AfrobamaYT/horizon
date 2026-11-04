//! Injection points (#589): the game's `#[point]` methods, read from its source like the hooks'
//! functions, as the slot statics the launcher fills (`points_generated.rs`) and the list it
//! routes injections by (`points.json`). Both are build outputs, never committed.
#[path = "../../horizon-mod-api-macros/src/point_shape.rs"]
mod point_shape;

use quote::{ToTokens, quote};
use std::{collections::BTreeMap, path::Path};

/// One point, as `points.json` lists it.
#[derive(Debug, serde::Serialize)]
pub struct GamePoint {
    pub name: String,
    /// The function, as an injection's `target` (and a hook's) names it.
    pub target: String,
    pub side: &'static str,
    /// The listener's host: `host` (`&self`), or the side's writing host (`&mut self`).
    pub host: &'static str,
    pub hot: bool,
    /// The parameters passed, with their types, in order.
    pub args: Vec<(String, String)>,
    #[serde(skip)]
    listener: String,
}

/// The `#[point]` methods of `Game`, in `game.rs` and under `game/`, by name. Two with one name
/// are refused, naming both: the second takes `#[point(name = "…")]`.
pub fn game_points(root: &Path) -> Result<Vec<GamePoint>, String> {
    let src = root.join("crates/horizon-client/src");
    let mut methods = Vec::new();
    super::file_game_methods(&src.join("game.rs"), "horizon_client::game", &mut methods)?;
    super::game_methods(&src.join("game"), "horizon_client::game", &mut methods)?;
    let mut points = BTreeMap::<String, GamePoint>::new();
    for (module, method) in methods {
        let Some(attribute) = method.attrs.iter().find(|attr| {
            attr.path().segments.last().is_some_and(|segment| segment.ident == "point")
        }) else {
            continue;
        };
        let tokens = match &attribute.meta {
            syn::Meta::List(list) => list.tokens.clone(),
            _ => Default::default(),
        };
        let target = format!("{module}::Game<'_>::{}", method.sig.ident);
        let shape = point_shape::shape(tokens, &method.sig).map_err(|e| format!("{target}: {e}"))?;
        let point = GamePoint {
            target,
            side: shape.side.name(),
            host: match (shape.mutable, shape.side) {
                (false, _) => "host",
                (true, side) => side.name(),
            },
            hot: shape.hot,
            args: shape
                .args
                .iter()
                .map(|arg| (arg.name.to_string(), arg.ty.to_token_stream().to_string().replace("& ", "&")))
                .collect(),
            listener: shape.listener().to_string(),
            name: shape.name,
        };
        if let Some(other) = points.get(&point.name) {
            return Err(format!(
                "{} and {} are both the point {:?}: give one #[point(name = \"…\")]",
                other.target, point.target, point.name
            ));
        }
        points.insert(point.name.clone(), point);
    }
    Ok(points.into_values().collect())
}

/// The game's `points` module: a private slot per point and the one exported table of them, so a
/// Windows DLL spends one export on all of them.
pub fn slots(points: &[GamePoint]) -> String {
    let statics = points.iter().map(|point| {
        let slot = syn::Ident::new(&point.name.to_uppercase(), proc_macro2::Span::call_site());
        let listener: proc_macro2::TokenStream = point.listener.parse().expect("generated listener type");
        let target = &point.target;
        quote!(pub(crate) static #slot: Point<#listener> = Point::new(#target);)
    });
    let slots = points
        .iter()
        .map(|point| syn::Ident::new(&point.name.to_uppercase(), proc_macro2::Span::call_site()));
    quote! {
        use mod_api::__private::point::{Attach, Point};
        #(#statics)*
        /// Every point, for the launcher's generated `main` to fill by target.
        pub static POINTS: &[&(dyn Attach + Sync)] = &[#(&#slots),*];
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(attribute: &str, method: &str) -> Result<point_shape::Shape, String> {
        let method: syn::ImplItemFn = syn::parse_str(method).map_err(|e| e.to_string())?;
        point_shape::shape(attribute.parse().unwrap(), &method.sig).map_err(|e| e.to_string())
    }

    #[test]
    fn a_point_passes_only_api_typed_parameters() {
        let shape = point(
            r#"side = "client""#,
            "fn turn(&mut self, mut yaw: f64, button: MouseButton, text: &str, batch: &mut HudBatch) {}",
        )
        .unwrap();
        let args: Vec<_> = shape.args.iter().map(|arg| arg.name.to_string()).collect();
        assert_eq!(args, ["yaw", "text"]);
        assert_eq!(
            shape.listener().to_string(),
            quote!(fn(&mut dyn ::mod_api::ClientHost, &mut (f64, &str,))).to_string()
        );
        let read = point(r#"side = "server", name = "reach_of""#, "fn reach(&self) -> f64 { 0.0 }").unwrap();
        assert_eq!((read.name.as_str(), read.slot().to_string()), ("reach_of", "REACH_OF".into()));
        assert_eq!(read.listener().to_string(), quote!(fn(&dyn ::mod_api::Host, &mut ())).to_string());
    }

    #[test]
    fn a_point_needs_a_side_and_a_game_receiver() {
        let refused = |attribute, method| point(attribute, method).err().expect("refused");
        assert!(refused("", "fn tick(&mut self) {}").contains("side"));
        assert!(refused(r#"side = "both""#, "fn free(x: u8) {}").contains("method"));
        assert!(refused(r#"side = "up""#, "fn tick(&mut self) {}").contains("server, client or both"));
    }

    #[test]
    fn two_points_with_one_name_are_refused_naming_both() {
        let root = std::env::temp_dir().join(format!("horizon-points-{}", std::process::id()));
        let src = root.join("crates/horizon-client/src");
        std::fs::create_dir_all(src.join("game")).unwrap();
        std::fs::write(
            src.join("game.rs"),
            "impl Game<'_> { #[mod_api::point(side = \"both\")] fn tick(&mut self) {} }",
        )
        .unwrap();
        std::fs::write(
            src.join("game/remote.rs"),
            "impl Game<'_> { #[point(side = \"client\")] fn tick(&mut self, input: u8) {} fn plain(&self) {} }",
        )
        .unwrap();
        let error = game_points(&root).unwrap_err();
        assert!(error.contains("horizon_client::game::Game<'_>::tick"), "{error}");
        assert!(error.contains("horizon_client::game::remote::Game<'_>::tick"), "{error}");
        std::fs::write(
            src.join("game/remote.rs"),
            "impl Game<'_> { #[point(side = \"client\", name = \"remote_tick\")] fn tick(&mut self, input: u8) {} }",
        )
        .unwrap();
        let points = game_points(&root).unwrap();
        let targets: Vec<_> = points.iter().map(|p| (p.name.as_str(), p.target.as_str())).collect();
        assert_eq!(
            targets,
            [
                ("remote_tick", "horizon_client::game::remote::Game<'_>::tick"),
                ("tick", "horizon_client::game::Game<'_>::tick"),
            ]
        );
        assert!(slots(&points).contains("POINTS"));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
