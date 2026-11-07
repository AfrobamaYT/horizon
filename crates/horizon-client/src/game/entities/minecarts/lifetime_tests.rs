use super::*;
use horizon_core::nbt::Tag;
use horizon_mod_runtime::attachments::{
    AttachmentType, Attachments, HasAttachmentLifetime, Holder, index::AttachmentIndex,
};
use std::sync::Mutex;

static VALUE: AttachmentType<i32> = AttachmentType {
    namespace: "test",
    name: "cart_lifetime",
    default: || 0,
    codec: None,
    copy_on_death: false,
    sync: false,
};

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_bound_cart_commits_the_normal_server_tick() {
    bound_cart_case(CartKind::Furnace, |game| {
        game.carts.update(0, |cart| cart.fuel = 6);
        game.tick_carts();
        assert_eq!(game.carts[0].fuel, 5);
        assert_eq!(game.carts[0].tick_count, 1);
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_bound_cart_commits_the_ridden_tick() {
    bound_cart_case(CartKind::Rideable, |game| {
        game.carts.update(0, |cart| cart.in_water = true);
        game.player = Some(
            horizon_client::motion::PlayerMotion::new(
                game.motion,
                Vec3::new(0.5, 70.0 + SEAT, 0.5),
            )
            .unwrap(),
        );
        game.riding = Some(71);
        assert!(game.tick_cart_riding(Keys::default()));
        assert!(!game.carts[0].in_water);
        assert_eq!(game.carts[0].tick_count, 1);
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_bound_ticked_cart_publishes_its_moved_position_before_pushing() {
    bound_cart_case(CartKind::Rideable, |game| {
        game.carts.update(0, |cart| cart.velocity.x = 0.4);
        game.carts.push(Minecart::new(
            72,
            CartKind::Rideable,
            Vec3::new(1.75, 70.0, 0.5),
        ));
        // Keep the other cart's own tick from masking the first cart's push.
        game.riding = Some(72);
        game.tick_carts();
        assert!(game.carts[0].position.x > 0.5);
        assert!(game.carts[1].velocity.x > 0.0);
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_bound_cart_commits_the_tnt_interaction() {
    bound_cart_case(CartKind::Tnt, |game| {
        game.inventory[game.selected] = Some(ItemStack::new("minecraft:fire_charge", 1));
        assert!(game.interact_cart(Some(InteractTarget {
            id: 71,
            location: [0.5, 70.0, 0.5],
        })));
        assert_eq!(game.carts[0].fuse, 80);
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_bound_cart_commits_valid_nbt_and_refuses_an_invalid_load() {
    bound_cart_case(CartKind::Furnace, |game| {
        let target = super::super::commands::Target::Cart(71);
        game.carts.update(0, |cart| cart.fuel = 6);
        let uuid = game.uuid_for(71);
        assert!(
            game.apply_entity_nbt(target, Tag::Compound(Default::default()))
                .is_err()
        );
        assert_eq!(game.carts[0].fuel, 6);
        let (tag, _) = horizon_core::nbt::parse_snbt(
            "{Pos:[0.5d,70.0d,0.5d],Fuel:13s,UUID:[I;9,8,7,6],DisplayState:{Name:\"test:missing\"}}",
        ).unwrap();
        game.apply_entity_nbt(target, tag).unwrap();
        assert_eq!(game.carts[0].fuel, 13);
        assert_eq!(game.uuid_for(71), uuid);
        assert!(game.carts[0].custom_display.is_none());
    });
}

fn bound_cart_case(kind: CartKind, action: impl FnOnce(&mut Game<'_>)) {
    super::super::native_entity_refs::tests::with_game(|game| {
        game.tickets.update(None);
        game.carts
            .push(Minecart::new(71, kind, Vec3::new(0.5, 70.0, 0.5)));
        let holder = {
            let host: &dyn mod_api::ServerHost = game;
            host.entity(71).unwrap()
        };
        let index = Arc::new(Mutex::new(AttachmentIndex::default()));
        let mut values = Attachments::default();
        VALUE.set(&mut values, Holder::Entity(71), 10).unwrap();
        let original = game.carts.update(0, |cart| {
            cart.attachment_lifetime().bind(&mut values, &index, holder);
            cart.attachment_lifetime().registration().unwrap()
        });
        let key = ("test".into(), "cart_lifetime".into());
        let before = index.lock().unwrap().snapshot(&key);
        assert_eq!(before.len(), 1);
        action(game);
        let current = game
            .carts
            .update(0, |cart| cart.attachment_lifetime().registration().unwrap());
        assert!(original.same_registration(&current));
        assert_eq!(index.lock().unwrap().snapshot(&key).len(), 1);
        assert_eq!(VALUE.get(&mut values, Holder::Entity(71)), Ok(10));
        let taken = game.take_entities();
        assert_eq!(index.lock().unwrap().snapshot(&key).len(), 1);
        game.restore_entities(taken);
        VALUE.set(&mut values, Holder::Entity(71), 20).unwrap();
        let after = index.lock().unwrap().snapshot(&key);
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].holder, holder);
        assert_eq!(after[0].entry.read(&VALUE), Ok(&20));
        game.persist
            .removals
            .push(super::super::commands::Target::Cart(71));
        let clone = game.carts[0].clone();
        assert!(!game.store_cart_back(0, clone));
        game.native_removals_flush();
        assert!(!game.carts.iter().any(|cart| cart.id == 71));
        assert!(index.lock().unwrap().snapshot(&key).is_empty());
        VALUE.set(&mut values, Holder::Entity(71), 30).unwrap();
        assert!(index.lock().unwrap().snapshot(&key).is_empty());
        assert_eq!(before[0].entry.read(&VALUE), Ok(&10));
        assert_eq!(after[0].entry.read(&VALUE), Ok(&20));
    });
}
