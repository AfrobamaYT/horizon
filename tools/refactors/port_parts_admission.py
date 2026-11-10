#!/usr/bin/env python3
"""Carry newly merged authoring through the committed verbatim Parts extractor."""
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

root = Path(sys.argv[1]).resolve()
extractor = Path(sys.argv[2]).resolve()
vocab = root / 'crates/horizon-mod-vocab/src/parts'
api = root / 'crates/horizon-mod-api/src/parts'
for name in ('registration_refusal.rs', 'registration_state.rs'):
    if (api / name).exists():
        raise SystemExit(f'authoring destination already exists: {name}')
(root / '.local').mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix='parts-admission-', dir=root / '.local') as directory:
    temporary = Path(directory)
    source = temporary / 'crates/horizon-mod-vocab/src/parts'
    destination = temporary / 'crates/horizon-mod-api/src/parts'
    source.mkdir(parents=True)
    for name, original in (
        ('menus.rs', vocab / 'menus.rs'),
        ('registration_refusal.rs', vocab / 'registration_refusal.rs'),
        ('registration_state.rs', vocab / 'shared/registration_state.rs'),
    ):
        shutil.copyfile(original, source / name)
    (source / 'mod.rs').write_text('impl Registrar for PartRegistrar {}\n')
    subprocess.run([str(extractor), str(temporary)], check=True)
    for name, original in (
        ('menus.rs', vocab / 'menus.rs'),
        ('registration_refusal.rs', vocab / 'registration_refusal.rs'),
        ('registration_state.rs', vocab / 'shared/registration_state.rs'),
    ):
        shutil.copyfile(source / name, original)
        shutil.copyfile(destination / name, api / name)
book_tests = '#[cfg(test)]\n#[path = "menus/book_tests.rs"]\nmod book_tests;\n'
path = vocab / 'menus.rs'
text = path.read_text()
if text.count(book_tests) != 1:
    raise SystemExit('expected exactly one incoming recipe-book test module')
path.write_text(text.replace(book_tests, ''))
path = api / 'menus.rs'
path.write_text(path.read_text().replace('use super::*;\n', 'use super::*;\n' + book_tests, 1))
(api / 'menus').mkdir(exist_ok=True)
shutil.move(vocab / 'menus/book_tests.rs', api / 'menus/book_tests.rs')
