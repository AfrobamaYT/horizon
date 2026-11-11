use crate::{internals, sites};
use std::path::Path;

/// Production catalog proof; deliberately opt-in because catalog traversal is expensive.
#[test]
#[ignore]
fn production_listed_records() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let entries = internals::catalog(&root).unwrap();
    let slots = sites::slots(&root.join(sites::DIRECTORY)).unwrap();
    for (file, module, suffix, ordinal, once) in [
        (
            "crates/horizon-client/src/mobs/villager_secondary.rs",
            "horizon_client_lib::mobs::villager_brain::secondary",
            "sense_secondary",
            0,
            false,
        ),
        (
            "crates/horizon-client/src/game/interface/sign_edit.rs",
            "horizon_client::game::sign_edit",
            "draw_sign_editor",
            0,
            false,
        ),
        (
            "crates/horizon-client/src/game/interface/sign_edit.rs",
            "horizon_client::game::sign_edit",
            "draw_sign_editor",
            1,
            false,
        ),
        (
            "crates/horizon-client/src/game/blocks/machines/mod.rs",
            "horizon_client::game::machines",
            "dispense_from",
            0,
            true,
        ),
    ] {
        let declaration = slots
            .calls
            .iter()
            .find(|d| d.target.ends_with(suffix) && d.ordinal == Some(ordinal))
            .unwrap();
        let source = std::fs::read_to_string(root.join(file)).unwrap();
        let (listed, metadata) = sites::rewrite::rewrite(
            &source,
            module,
            &entries,
            std::slice::from_ref(declaration),
            false,
        )
        .unwrap();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].once, once);
        assert_eq!(metadata[0].ctx, declaration.ctx);
        let (rebuilt, _) = sites::rewrite::rewrite(
            &source,
            module,
            &entries,
            std::slice::from_ref(declaration),
            true,
        )
        .unwrap();
        assert_eq!(listed, rebuilt, "same listener and record on both routes");
        println!("{}", serde_json::to_string_pretty(&metadata).unwrap());
    }
}
