"""Census of the mod API's public structs by construction kind (#2862).

Every public struct in `horizon-mod-api` and `horizon-mod-vocab` is one of two
kinds:

1. built by mods: public fields and `Default`, or a constructor taking the
   required fields;
2. only received by mods: `#[non_exhaustive]`.

A struct that a mod can build with a brace literal (named fields, all `pub`,
not `#[non_exhaustive]`) but that has neither `Default` nor a constructor is
"neither": adding a field to it breaks every mod that builds it. This script
lists those from source, with no hand-kept list. `NEITHER_LIMIT` in
`tests/test_mod_api_struct_kinds.py` is how many may remain; it only goes down.

Run: `python tools/mod_api_struct_kinds.py` (lists the "neither" structs),
`--summary` (counts only).
"""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools.check_mod_api_wire import code_tokens  # noqa: E402

CRATES = ('horizon-mod-api', 'horizon-mod-vocab')


def _skip_group(ts, i, open_, close):
    """Index just past the group opened at `i`."""
    depth = 0
    while i < len(ts):
        depth += (ts[i] == open_) - (ts[i] == close)
        i += 1
        if not depth:
            break
    return i


def _attributes(ts, i):
    """The attribute tokens attached before index `i` (a `pub`)."""
    j, attrs = i - 1, []
    while j >= 0 and ts[j] == ']':
        k, depth = j, 0
        while k >= 0:
            depth += (ts[k] == ']') - (ts[k] == '[')
            if not depth:
                break
            k -= 1
        if k < 1 or ts[k - 1] != '#':
            break
        attrs = ts[k - 1:j + 1] + attrs
        j = k - 2
    return attrs


def _fields_public(body):
    """True when every named field in a struct body is plain `pub`."""
    depth, expect_field, public = 0, True, True
    i = 0
    while i < len(body):
        t = body[i]
        if t in ('(', '[', '{', '<'):
            depth += 1
        elif t in (')', ']', '}', '>'):
            depth -= 1
        elif depth == 0 and t == ',':
            expect_field = True
        elif depth == 0 and expect_field and t == '#':
            i = _skip_group(body, i + 1, '[', ']')
            continue
        elif depth == 0 and expect_field:
            expect_field = False
            # `pub name:` is public; `pub(crate) name:` and `name:` are not.
            if not (t == 'pub' and i + 2 < len(body) and body[i + 2] == ':'):
                public = False
        i += 1
    return public


def structs(source):
    """(name, line, non_exhaustive, derives_default, literal) per public struct."""
    pairs = code_tokens(source)
    ts = [v for v, _ in pairs]
    # A `mod built { .. }` holds the game's own literal-built mirrors, reached
    # only through `__private::built`: not part of what mods build.
    mirrors = set()
    for i in range(len(ts) - 2):
        if ts[i] == 'mod' and ts[i + 1] == 'built' and ts[i + 2] == '{':
            mirrors.update(range(i, _skip_group(ts, i + 2, '{', '}')))
    out = []
    for i in range(len(ts) - 2):
        if ts[i] != 'pub' or ts[i + 1] != 'struct' or i in mirrors:
            continue
        name = ts[i + 2]
        if not re.match(r'^[A-Za-z_]\w*$', name):
            continue  # a macro template, not a declaration
        attrs = _attributes(ts, i)
        non_exhaustive = 'non_exhaustive' in attrs
        derives_default = any(
            attrs[k] == 'derive' and 'Default' in attrs[k:_skip_group(attrs, k + 1, '(', ')')]
            for k in range(len(attrs))
        )
        j = i + 3
        if j < len(ts) and ts[j] == '<':
            j = _skip_group(ts, j, '<', '>')
        while j < len(ts) and ts[j] not in ('{', ';', '('):
            j += 1  # a where clause
        literal = False
        if j < len(ts) and ts[j] == '{':
            end = _skip_group(ts, j, '{', '}')
            literal = _fields_public(ts[j + 1:end - 1])
        line = source.count('\n', 0, pairs[i][1]) + 1
        out.append((name, line, non_exhaustive, derives_default, literal))
    return out


def defaults_and_constructors(source):
    """Names with a hand-written `Default` impl, and names with a constructor
    (a `pub fn` without a `self` receiver that returns `Self` or the type)."""
    ts = [v for v, _ in code_tokens(source)]
    defaults, constructors = set(), set()
    for i, t in enumerate(ts):
        if t != 'impl':
            continue
        j = i + 1
        if j < len(ts) and ts[j] == '<':
            j = _skip_group(ts, j, '<', '>')
        head = []
        while j < len(ts) and ts[j] != '{':
            head.append(ts[j])
            j += 1
        if 'for' in head:
            trait, target = head[:head.index('for')], head[head.index('for') + 1:]
            if trait and trait[-1] == 'Default' and target:
                defaults.add(target[0])
            continue
        if not head:
            continue
        name = head[0]
        end = _skip_group(ts, j, '{', '}')
        body = ts[j + 1:end - 1]
        for k in range(len(body) - 1):
            if body[k] == 'pub' and body[k + 1] == 'fn':
                params_at = body.index('(', k)
                params_end = _skip_group(body, params_at, '(', ')')
                params = body[params_at:params_end]
                if 'self' in params[:4]:
                    continue
                ret = []
                m = params_end
                if m < len(body) and body[m] == '-':
                    m += 2
                    while m < len(body) and body[m] not in ('{', 'where', ';'):
                        ret.append(body[m])
                        m += 1
                if ret[:1] in (['Self'], [name]) or ret[:3] in (
                        ['Option', '<', 'Self'], ['Result', '<', 'Self']):
                    constructors.add(name)
    return defaults, constructors


def sources(root=ROOT):
    found = {}
    for crate in CRATES:
        for path in sorted((root / 'crates' / crate / 'src').rglob('*.rs')):
            if path.name == '__private.rs' or path.name.endswith('_tests.rs'):
                continue
            if crate == 'horizon-mod-vocab' and path.parent.name == 'src' and path.name == 'built.rs':
                continue  # the `__private::built` mirrors, as `mod built` blocks are
            found[str(path.relative_to(root)).replace('\\', '/')] = path.read_text(encoding='utf-8')
    return found


def census(root=ROOT):
    """Counts by kind, and the "neither" structs as `path:line: Name`."""
    files = sources(root)
    defaults, constructors = set(), set()
    for source in files.values():
        d, c = defaults_and_constructors(source)
        defaults |= d
        constructors |= c
    counts = {'public': 0, 'non_exhaustive': 0, 'literal': 0, 'default': 0,
              'constructor': 0, 'neither': 0}
    neither = []
    for path, source in files.items():
        for name, line, non_exhaustive, derives_default, literal in structs(source):
            counts['public'] += 1
            if non_exhaustive:
                counts['non_exhaustive'] += 1
                continue
            if not literal:
                continue
            counts['literal'] += 1
            if derives_default or name in defaults:
                counts['default'] += 1
            elif name in constructors:
                counts['constructor'] += 1
            else:
                counts['neither'] += 1
                neither.append(f'{path}:{line}: {name}')
    return counts, neither


if __name__ == '__main__':
    counts, neither = census()
    if '--summary' not in sys.argv:
        for entry in neither:
            print(entry)
    print(', '.join(f'{k} {v}' for k, v in counts.items()))
