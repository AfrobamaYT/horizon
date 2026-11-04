"""The mod API's boundary: mods see only the API, and the game reaches the rest through
`mod_api::__private`."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
API = ROOT / "crates/horizon-mod-api/src"
def api_files():
    return [p for p in sorted(API.rglob("*.rs")) if p.name != "__private.rs" and not p.name.endswith("_tests.rs")]


def api_text():
    return "\n".join(p.read_text() for p in api_files())


def preceding(lines, i):
    """The attribute, doc and comment lines directly above line `i`, nearest first."""
    found = []
    while i > 0 and lines[i - 1].strip().startswith(("#[", "//")):
        i -= 1
        found.append(lines[i].strip())
    return found


class ModApiBoundaryTests(unittest.TestCase):
    def test_every_public_type_is_non_exhaustive_or_says_why(self):
        missing = []
        for path in api_files():
            lines = path.read_text().splitlines()
            for i, line in enumerate(lines):
                if re.match(r"\s*pub (struct|enum) \w+", line):
                    above = preceding(lines, i)
                    if not any(a == "#[non_exhaustive]" or a.startswith("// exhaustive: ") for a in above):
                        missing.append(f"{path.relative_to(ROOT)}:{i + 1}: {line.strip()}")
        self.assertEqual(missing, [], "add #[non_exhaustive], or a `// exhaustive: <why>` line above the type")

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

    def test_doc_hidden_outside_private_says_why(self):
        # `__private` is the game's side. Elsewhere an item is hidden only with a
        # `// hidden: <why>` line above, except a macro re-export or a crate-private module.
        unexplained = []
        for path in api_files():
            lines = path.read_text().splitlines()
            for i, line in enumerate(lines):
                if line.strip() != "#[doc(hidden)]" or i + 1 >= len(lines):
                    continue
                item = lines[i + 1].strip()
                if item.startswith(("pub(crate)", "pub use horizon_mod_api_macros::")):
                    continue
                if not any(a.startswith("// hidden: ") for a in preceding(lines, i)):
                    unexplained.append(f"{path.relative_to(ROOT)}:{i + 1}: {item}")
        self.assertEqual(unexplained, [], "move the item into `__private`, or add `// hidden: <why>`")

    def test_unhandled_arms_name_a_non_exhaustive_enum(self):
        text = api_text()
        for path in ROOT.glob("crates/horizon-client*/src/**/*.rs"):
            for name in re.findall(r'unreachable!\("(\w+): unhandled variant"\)', path.read_text()):
                self.assertRegex(
                    text,
                    rf"#\[non_exhaustive\]\s*(?:#\[[^\n]*\]\s*)*pub enum {name}\b",
                    f"{path.relative_to(ROOT)}: `{name}` is not a #[non_exhaustive] pub enum of the API",
                )


if __name__ == "__main__":
    unittest.main()
