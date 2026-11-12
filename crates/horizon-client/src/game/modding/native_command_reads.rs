//! What a server mod reads beyond a command's arguments (NeoForge's
//! `ICommandSourceStackExtension`): where the command executes, a player's game profile and a
//! loaded advancement.
use super::Game;
use super::api_values::TryIntoApi;
use horizon_client::advancements::{Advancement, Frame};
use mod_api::{
    AdvancementDisplay, AdvancementFrame, AdvancementInfo, CommandArgument, CommandInvocation,
    GameProfile, Identifier, PlayerRef, ProfileProperty,
};

impl Game<'_> {
    /// A parsed native command with the level and position it executes at: after `execute
    /// in/at/positioned`, or the console's overworld spawn corner.
    pub(super) fn native_invocation(
        &self,
        literals: Vec<String>,
        arguments: Vec<CommandArgument>,
    ) -> CommandInvocation {
        CommandInvocation::new(
            literals,
            arguments,
            self.native_level_key(),
            self.source_position(),
        )
    }

    /// `ServerPlayer.getGameProfile`: the name `player_name` reads, and the properties the
    /// player logged in with. None for a stale or unavailable player.
    pub(super) fn player_profile(&self, player: PlayerRef) -> Option<GameProfile> {
        let name = self.player_name(player)?;
        let properties = self
            .lan
            .as_ref()
            .and_then(|host| {
                host.hub
                    .online_players()
                    .find(|guest| guest.uuid == player.uuid())
            })
            .map(|guest| {
                guest
                    .properties
                    .iter()
                    .map(|p| {
                        ProfileProperty::new(p.name.clone(), p.value.clone(), p.signature.clone())
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(GameProfile::new(player.uuid(), name, properties))
    }

    /// `ServerAdvancementManager.get(id)` over the loaded tree, read once on first use and
    /// replaced by `/reload`.
    pub(super) fn advancement(&mut self, id: &Identifier) -> Option<AdvancementInfo> {
        let tree = self.advancement_tree();
        advancement_info(&tree.list[*tree.by_id.get(&id.to_string())?])
    }
}

/// A loaded advancement as the API's snapshot. None when its id, parent or texts don't cross,
/// which a tree built from valid files never holds.
fn advancement_info(advancement: &Advancement) -> Option<AdvancementInfo> {
    let display = match &advancement.display {
        Some(d) => Some(AdvancementDisplay::new(
            horizon_client::scoreboard::comp_text(&d.title)
                .try_into_api()
                .ok()?,
            horizon_client::scoreboard::comp_text(&d.description)
                .try_into_api()
                .ok()?,
            Identifier::parse(&d.icon).ok()?,
            match d.frame {
                Frame::Task => AdvancementFrame::Task,
                Frame::Goal => AdvancementFrame::Goal,
                Frame::Challenge => AdvancementFrame::Challenge,
            },
            d.show_toast,
            d.announce,
            d.hidden,
        )),
        None => None,
    };
    let parent = match &advancement.parent {
        Some(parent) => Some(Identifier::parse(parent).ok()?),
        None => None,
    };
    Some(AdvancementInfo::new(
        Identifier::parse(&advancement.id).ok()?,
        parent,
        display,
        advancement
            .criteria
            .iter()
            .map(|c| c.name.clone())
            .collect(),
        advancement.requirements.clone(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_client::advancements::{Display, Rewards};

    #[test]
    fn a_loaded_advancement_crosses_with_its_parent_frame_and_criteria() {
        let advancement = Advancement {
            id: "minecraft:story/mine_stone".into(),
            parent: Some("minecraft:story/root".into()),
            display: Some(Display {
                icon: "minecraft:wooden_pickaxe".into(),
                title: serde_json::json!({"translate": "advancements.story.mine_stone.title"}),
                description: serde_json::json!("Mine Stone"),
                frame: Frame::Task,
                background: None,
                show_toast: true,
                announce: true,
                hidden: false,
            }),
            criteria: Vec::new(),
            requirements: vec![vec!["get_stone".into()]],
            rewards: Rewards::default(),
        };
        let info = advancement_info(&advancement).unwrap();
        assert_eq!(
            info.id,
            Identifier::parse("minecraft:story/mine_stone").unwrap()
        );
        assert_eq!(
            info.parent,
            Some(Identifier::parse("minecraft:story/root").unwrap())
        );
        let display = info.display.unwrap();
        assert_eq!(display.frame, AdvancementFrame::Task);
        assert_eq!(
            display.icon,
            Identifier::parse("minecraft:wooden_pickaxe").unwrap()
        );
        assert_eq!(info.requirements, vec![vec!["get_stone".to_owned()]]);
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_profile_keeps_its_login_properties() {
        super::super::native_guest_turn_tests::with_guests(|game| {
            let stored = horizon_protocol::packets::login::Property {
                name: "textures".into(),
                value: "e30=".into(),
                signature: Some("sig".into()),
            };
            game.lan
                .as_mut()
                .unwrap()
                .hub
                .update_guest(12, |guest| guest.properties = vec![stored])
                .unwrap();
            let player = game.player(12).unwrap();
            assert_eq!(
                game.player_profile(player),
                Some(GameProfile::new(
                    12,
                    "guest12".into(),
                    vec![ProfileProperty::new(
                        "textures".into(),
                        "e30=".into(),
                        Some("sig".into())
                    )],
                ))
            );
        });
    }
}
