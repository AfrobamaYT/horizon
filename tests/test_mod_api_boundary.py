"""The mod API's boundary: mods see only the API, and the game reaches the rest through
`mod_api::__private`. Written by tools/split_mod_api.py; rerunning it updates the numbers."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
API = ROOT / "crates/horizon-mod-api/src"
# `#[non_exhaustive]` on the API's types when the module became the crate: none is dropped.
NON_EXHAUSTIVE_FLOOR = 84
# `#[doc(hidden)] pub` items of the API outside `__private`, as the module had them: no more.
DOC_HIDDEN_CEILING = 62
# The game's matches on API enums that it can't complete, by enum and file. A variant added to
# one of these enums is handled at each place listed, or the game panics there.
UNHANDLED = [
    ('EffectTickChange', 'crates/horizon-client/src/game/potions.rs'),
    ('MenuSlotBacking', 'crates/horizon-client/src/game/native_menu_refs.rs'),
    ('NativeArrowPickup', 'crates/horizon-client/src/game/native_projectile_refs.rs'),
    ('NativeBlockFace', 'crates/horizon-client/src/game/native_entity_refs.rs'),
    ('NativeBlockShapeKind', 'crates/horizon-client/src/game/native_entity_refs.rs'),
    ('NativeClipBlock', 'crates/horizon-client/src/game/native_entity_refs.rs'),
    ('NativeClipFluid', 'crates/horizon-client/src/game/native_entity_refs.rs'),
    ('NativeEffectDamage', 'crates/horizon-client/src/mobs.rs'),
    ('NativeExplosionInteraction', 'crates/horizon-client/src/game/native_level_events.rs'),
    ('NativeSupportType', 'crates/horizon-client/src/game/native_entity_refs.rs'),
    ('PhantomSpawnResult', 'crates/horizon-client/src/game/stats.rs'),
    ('ToolAbility', 'crates/horizon-client/src/game.rs'),
    ('UsePhase', 'crates/horizon-client/src/game/remote/interaction_event_tests.rs'),
]


def api_text():
    return "\n".join(p.read_text() for p in sorted(API.rglob("*.rs")) if p.name != "__private.rs")


class ModApiBoundaryTests(unittest.TestCase):
    def test_no_api_type_loses_non_exhaustive(self):
        self.assertGreaterEqual(api_text().count("#[non_exhaustive]"), NON_EXHAUSTIVE_FLOOR)

    def test_the_game_side_is_reached_only_through_private(self):
        def names(text):
            found = set()
            for path in re.findall(r"^\s*pub use ([^;]*);", text, re.M):
                if "{" in path:
                    found |= {n.split(" as ")[-1].split("::")[-1].strip()
                              for n in re.sub(r"[{}\s]", " ", path.split("{", 1)[1]).split(",") if n.strip()}
                else:
                    found.add(path.split(" as ")[-1].split("::")[-1].strip())
            return found
        # `built` holds copies under the API's own names, so it is left out.
        private = re.sub(r"\npub mod built \{.*?\n\}\n", "\n", (API / "__private.rs").read_text(), flags=re.S)
        game = names(private) - {"*"}
        api = names((API / "lib.rs").read_text())
        self.assertEqual(api & game, set(), "a game-side item is re-exported as API")
        # No item is opened by hiding it: the game's side grows in `__private` only.
        self.assertLessEqual(len(re.findall(r"#\[doc\(hidden\)\]\s*pub\b", api_text())), DOC_HIDDEN_CEILING)

    def test_unhandled_arms_are_listed(self):
        found = set()
        for path in ROOT.glob("crates/horizon-client*/src/**/*.rs"):
            for name in re.findall(r'unreachable!\("(\w+): unhandled variant"\)', path.read_text()):
                found.add((name, str(path.relative_to(ROOT))))
        self.assertEqual(found, set(UNHANDLED))
        for name, _ in UNHANDLED:
            self.assertRegex(api_text(), rf"pub enum {name}\b")


if __name__ == "__main__":
    unittest.main()
