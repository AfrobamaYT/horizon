#!/usr/bin/env python3
"""Move the lower-crate replacement declarations onto their function definitions."""
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
listing = root / 'crates/horizon-mod-api-gen/replaceable.txt'
for target in listing.read_text().splitlines():
    if not target or target.startswith('#'):
        continue
    parts = re.sub(r'<[^>]*>', '', target).split('::')
    crate, name = parts[0], parts[-1]
    module = parts[1:-1]
    owner = module.pop() if module and module[-1][0].isupper() else None
    package = root / 'crates' / crate.replace('_', '-')
    manifest = package / 'Cargo.toml'
    text = manifest.read_text()
    if 'horizon-mod-api-macros' not in text:
        manifest.write_text(text.replace('[dependencies]', '[dependencies]\nhorizon-mod-api-macros = { path = "../horizon-mod-api-macros" }', 1))
    candidates = []
    for path in (root / 'crates').rglob('*.rs'):
        if '/src/' not in str(path):
            continue
        relative = str(path).split('/src/', 1)[1]
        prefix = '/'.join(module)
        if not (relative == prefix + '.rs' or relative == prefix + '/mod.rs' or relative.startswith(prefix + '/')):
            continue
        source = path.read_text()
        for match in re.finditer(r'(?m)^([ \t]*)(?:(?:pub(?:\([^)]*\))?|async|unsafe|const)\s+)*fn\s+' + name + r'\b', source):
            if owner and not re.search(r'impl[^{}]*\b' + owner + r'\b[^{}]*\{[^}]*$', source[:match.start()], re.S):
                # Methods following another method have closing braces; use the latest impl header.
                headers = list(re.finditer(r'impl[^{}]*\{', source[:match.start()]))
                if not headers or not re.search(r'\b' + owner + r'\b', headers[-1].group()):
                    continue
            candidates.append((path, match.start(), match.group(1)))
    # Re-exported client-lib modules live in the client tree; other crates win their own definitions.
    own = [c for c in candidates if str(c[0]).startswith(str(package) + '/')]
    candidates = own or candidates
    if len(candidates) != 1:
        raise RuntimeError((target, candidates))
    path, offset, indent = candidates[0]
    source = path.read_text()
    path.write_text(source[:offset] + indent + '#[horizon_mod_api_macros::replaceable]\n' + source[offset:])
