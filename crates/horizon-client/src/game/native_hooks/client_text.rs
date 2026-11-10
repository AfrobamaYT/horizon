use super::*;
use crate::game::api_values::{TryIntoApi, TryIntoEngine};

impl Game<'_> {
    pub(in crate::game) fn text_width(
        &self,
        text: &mod_api::Text,
    ) -> std::result::Result<i32, mod_api::text::TextFormError> {
        let text = text.try_into_engine(&())?;
        Ok(self.gui.atlas.font.width_text(&text) as i32)
    }

    pub(in crate::game) fn text_split(
        &self,
        text: &mod_api::Text,
        width: i32,
    ) -> std::result::Result<Vec<mod_api::Text>, mod_api::text::TextFormError> {
        let text = text.try_into_engine(&())?;
        self.gui
            .atlas
            .font
            .split_formatted(&text.resolved(), width as f32)
            .iter()
            .map(TryIntoApi::try_into_api)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn styled_text_reads_use_live_font_language_and_wrapping_outside_drawing() {
        crate::game::native_entity_refs::tests::with_game(|game| {
            let mut text = mod_api::Text::literal("AA AA");
            text.style.bold = Some(true);
            let form = mod_api::text::TextForm::try_from(&text).unwrap();
            let text = mod_api::Text::try_from(form).unwrap();
            let host: &dyn mod_api::ClientHost = game;
            assert_eq!(host.text_width(&mod_api::Text::literal("AA")).unwrap(), 12);
            assert_eq!(host.text_width(&mod_api::Text::literal("")).unwrap(), 0);
            let lines = host.text_split(&text, 14).unwrap();
            let plain: Vec<_> = lines
                .iter()
                .map(|line| {
                    let line: horizon_client::text::Text = line.try_into_engine(&()).unwrap();
                    assert!(line.chars().all(|(_, style)| style.is_bold()));
                    line.plain()
                })
                .collect();
            assert_eq!(plain, ["AA", "AA"]);
            assert_eq!(
                host.text_split(&mod_api::Text::literal("AA"), -1)
                    .unwrap()
                    .len(),
                2
            );

            let translated = mod_api::Text::new(mod_api::text::TextContent::Translatable {
                key: "test.measure".into(),
                fallback: None,
                arguments: vec![],
            });
            for (value, width) in [("A", 6), ("AA", 12)] {
                horizon_client::lang::scoped(
                    horizon_client::lang::Language::of(
                        "test",
                        [("test.measure".into(), value.into())],
                    ),
                    || {
                        assert_eq!(host.text_width(&translated).unwrap(), width);
                    },
                );
            }
            // A live replacement font uses explicit provider advances, not any
            // metrics cached by a previous host read.
            let source = horizon_assets::resource_source::ResourceSource::indexed(
                std::collections::BTreeMap::from([(
                    "assets/minecraft/font/default.json".into(),
                    horizon_assets::resource_source::Resource::bytes(
                        br#"{"providers":[{"type":"space","advances":{"A":4}}]}"#.to_vec(),
                    ),
                )]),
            );
            game.gui.atlas.font = horizon_client::font::Font::metrics(&source);
            let host: &dyn mod_api::ClientHost = game;
            assert_eq!(host.text_width(&mod_api::Text::literal("AA")).unwrap(), 8);
        });
    }
}
