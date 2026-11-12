use super::*;

fn glide(hand: usize, creative: bool, elytra: bool) {
    crate::game::native_guest_turn_tests::with_guests(|game| {
        let mut stack = ItemStack::new(
            if elytra {
                "minecraft:elytra"
            } else {
                "minecraft:stick"
            },
            1,
        );
        if !elytra {
            let components = if hand == 4 {
                "[minecraft:glider={},minecraft:equippable={slot:\"mainhand\"},minecraft:max_damage=10,minecraft:max_stack_size=1]"
            } else {
                "[minecraft:glider={},minecraft:equippable={slot:\"offhand\"},minecraft:max_damage=10,minecraft:max_stack_size=1]"
            };
            let tag = horizon_client::item_data::command_components(components).unwrap();
            let (_, extra) = horizon_client::item_data::decode_components(Some(&tag));
            stack.extra = extra;
        }
        let body = horizon_client::motion::PlayerMotion::new(
            game.motion,
            horizon_core::geometry::Vec3::new(3.0, 80.0, 5.0),
        )
        .unwrap();
        game.lan
            .as_mut()
            .unwrap()
            .hub
            .update_guest(12, |g| {
                g.game_mode = if creative { 1 } else { 0 };
                g.on_ground = false;
                g.fall_flying = true;
                g.motion = Some(Box::new(body));
                g.armor = Default::default();
                if elytra {
                    g.armor[1] = Some(stack);
                } else if hand == 4 {
                    g.inventory[g.selected] = Some(stack);
                } else {
                    g.offhand = Some(stack);
                }
            })
            .unwrap();
        for _ in 0..20 {
            game.guest_glide_tick(12, -1.0);
        }
        let g = game.lan.as_ref().unwrap().hub.guest(12).unwrap();
        let stack = if elytra {
            g.armor[1].as_ref()
        } else if hand == 4 {
            g.inventory[g.selected].as_ref()
        } else {
            g.offhand.as_ref()
        }
        .unwrap();
        assert_eq!(stack.damage, if creative { 0 } else { 1 });
    });
}

#[test]
#[ignore = "needs prepared resources"]
fn main_hand_glide_wear() {
    glide(4, false, false);
}
#[test]
#[ignore = "needs prepared resources"]
fn off_hand_glide_wear() {
    glide(5, false, false);
}
#[test]
#[ignore = "needs prepared resources"]
fn creative_glide_wear() {
    glide(4, true, false);
}
#[test]
#[ignore = "needs prepared resources"]
fn chest_glide_wear() {
    glide(1, false, true);
}
