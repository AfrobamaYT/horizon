#!/usr/bin/env python3
"""Adds one host method to an area file and its game forwarder, for helpers that landed on
main while the host split was in review (a `#[helper]` in `hooks/<area>.rs`).

Usage: add_method.py AREA TIER NAME 'SIGNATURE' ACCESS < doc
  AREA       area file stem (`camera_read`); created when missing
  TIER       Host | ServerHost | ClientHost | Backend | ClientBackend | BackendMut | ClientBackendMut
  NAME       method name; the game function is `native_<name>`
  SIGNATURE  `fn name(&self, ...) -> Ret` as the trait declares it
  ACCESS     LoadAccess variant the call needs (`WorldOnly`, `ClientSetup`, ...)
  stdin      the method's doc, one `///` line per line, without the slashes
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
API = ROOT / "crates/horizon-mod-api/src/host"
GAME = ROOT / "crates/horizon-client/src/game/host"
DESCRIPTION = {
    "Host": "The host's methods both sides share of the `{a}` area, implemented by the game and reached through [`Host`].",
    "ServerHost": "The server host's methods of the `{a}` area, implemented by the game and reached through [`ServerHost`].",
    "ClientHost": "The client host's methods of the `{a}` area, implemented by the game and reached through [`ClientHost`].",
    "Backend": "The handles' reads of the `{a}` area, implemented by the game and reached through [`Backend`].",
    "ClientBackend": "The handles' client reads of the `{a}` area, implemented by the game and reached through [`Backend`].",
    "BackendMut": "The handles' writes of the `{a}` area, implemented by the game and reached through [`BackendMut`].",
    "ClientBackendMut": "The handles' client writes of the `{a}` area, implemented by the game and reached through [`ClientBackendMut`].",
}
UNION = {"ClientBackend": "Backend"}


def camel(stem):
    return "".join(p.capitalize() for p in stem.split("_"))


def side(tier):
    return "client" if tier.startswith("Client") else "server" if tier == "ServerHost" else "both"


def main():
    area, tier, name, signature, access = sys.argv[1:6]
    doc = sys.stdin.read().rstrip("\n").split("\n")
    trait = camel(area) + tier
    union = UNION.get(tier, tier)
    api_path, game_path = API / f"{area}.rs", GAME / f"{area}.rs"
    method = f"    /// Side: {side(tier)}.\n" + "".join(f"    /// {line}\n" for line in doc)
    method += (
        f"    {signature} {{\n        crate::phase::unprovided(\n            \"{union}::{name}\",\n"
        f"            \"{name}\",\n            crate::phase::LoadAccess::{access},\n        )\n    }}\n"
    )
    hidden = "#[doc(hidden)]\n" if "Backend" in tier else ""
    if not api_path.exists():
        api_path.write_text(
            f"//! The `{area}` area: host methods the game implements (`game/host/{area}.rs`).\n"
            "#![allow(unused_variables)]\nuse crate::*;\n"
        )
        game_path.write_text(
            f"//! The game's side of the `{area}` area (`horizon-mod-api/src/host/{area}.rs`).\n"
            "use crate::game::Game;\nuse mod_api::__private::game::*;\nuse mod_api::{prelude::*, *};\n"
        )
    text = api_path.read_text()
    marker = f"pub trait {trait} {{\n"
    if marker in text:
        start = text.index(marker) + len(marker)
        end = text.index("\n}\n", start) + 1
        text = text[:end] + method + text[end:]
    else:
        text += f"/// {DESCRIPTION[tier].format(a=area)}\n{hidden}pub trait {trait} {{\n{method}}}\n"
    api_path.write_text(text)
    params = re.match(r"fn \w+\((.*?)\)(?: ->.*)?$", signature, re.S).group(1)
    args = ", ".join(
        part.split(":")[0].strip() for part in re.split(r",\s*(?![^<(\[]*[>)\]])", params) if ":" in part
    )
    forward = (
        f"    {signature} {{\n        mod_api::__private::check_access(\n            \"{name}\",\n"
        f"            mod_api::__private::LoadAccess::{access},\n        );\n        self.native_{name}({args})\n    }}\n"
    )
    text = game_path.read_text()
    marker = f"impl {trait} for Game<'_> {{\n"
    if marker in text:
        start = text.index(marker) + len(marker)
        end = text.index("\n}\n", start) + 1
        text = text[:end] + forward + text[end:]
    else:
        text += f"{marker}{forward}}}\n"
    game_path.write_text(text)


main()
