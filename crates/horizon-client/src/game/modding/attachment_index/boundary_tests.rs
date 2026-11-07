use super::*;
use crate::game::native_entity_refs::tests::with_game;
use mod_api::{AttachmentRowError, AttachmentType, AttachmentWriteToken};

static VALUE: AttachmentType<u32> = AttachmentType {
    namespace: "boundary",
    name: "value",
    default: || 0,
    codec: None,
    copy_on_death: false,
    sync: false,
};

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn current_dimension_capture_and_foreign_residency_validation() {
    with_game(|game| {
        VALUE
            .set(&mut game.world.attachments, Holder::Level, 7)
            .unwrap();
        game.native_level_opened(Dim::Overworld);
        let batch = game.attachment_query("boundary", "value");
        let forms = Vec::<mod_api::AttachmentRowForm>::from(&batch);
        let holder = forms[0].token.holder;
        let token = AttachmentWriteToken::from(forms[0].token.clone());
        assert_eq!(game.attachment_validate_write(token.clone()), Ok(()));
        assert_eq!(forms[0].entry.read(&VALUE), Ok(&7));

        let foreign = Arc::new(Mutex::new(AttachmentIndex::default()));
        let mut foreign_map = horizon_mod_runtime::attachments::Attachments::default();
        VALUE.set(&mut foreign_map, Holder::Level, 99).unwrap();
        foreign_map.bind_index(&foreign, holder);
        let original = game
            .attachment_indexes
            .insert(Dim::Overworld, foreign)
            .unwrap();
        assert_eq!(
            game.attachment_validate_write(token.clone()),
            Err(AttachmentRowError::StaleHolder)
        );
        assert_eq!(
            *game
                .attachment_query("boundary", "value")
                .query(&VALUE)
                .unwrap()
                .iter()
                .next()
                .unwrap()
                .unwrap()
                .value,
            99
        );
        game.attachment_indexes.insert(Dim::Overworld, original);
        assert_eq!(game.attachment_validate_write(token.clone()), Ok(()));

        game.dim = Dim::Nether;
        assert!(
            game.attachment_query("boundary", "value")
                .query(&VALUE)
                .unwrap()
                .iter()
                .next()
                .is_none()
        );
        assert_eq!(
            game.attachment_validate_write(token),
            Err(AttachmentRowError::StaleHolder)
        );
        game.dim = Dim::Overworld;
    });
}
