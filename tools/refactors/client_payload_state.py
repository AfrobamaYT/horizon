#!/usr/bin/env python3
"""Pass explicit connection payload state to private client receive functions.

Production calls share the session's snapshot; standalone network tests use the
immutable empty installation. Semantic protocol handling is written separately.
"""
from pathlib import Path
import re

path = Path("crates/horizon-client/src/net.rs")
source = path.read_text()
changes = []
tokens = re.compile(r'"(?:\\.|[^"\\])*"|//[^\n]*|/\*.*?\*/|[()]', re.S)
for match in re.finditer(r"\b(configure|receive_play)\s*\(", source):
    if re.search(r"\bfn\s+$", source[max(0, match.start()-8):match.start()]):
        continue
    depth = 1
    for token in tokens.finditer(source, match.end()):
        if token.group() == "(":
            depth += 1
        elif token.group() == ")":
            depth -= 1
            if depth == 0:
                if re.search(r"(?:\bpayloads|&crate::raw_payloads::Transport::empty\(\)),\s*$", source[match.end():token.start()]):
                    break
                line = source.rfind("\n", 0, token.start()) + 1
                indent = source[line:token.start()]
                if indent.strip():
                    raise ValueError("expected a multiline receive call")
                argument = "&crate::raw_payloads::Transport::empty()" if match.start() > source.index("#[cfg(test)]") else "payloads"
                changes.append((line, indent + "    " + argument + ",\n"))
                break
    else:
        raise ValueError("unclosed receive call")
for position, insertion in reversed(changes):
    source = source[:position] + insertion + source[position:]
source = re.sub(
    r"(\bSession\s*\{\s*\n(?P<indent>\s*)events,)(\n\s*writer:)",
    lambda match: match[1] + "\n" + match["indent"] + "payloads: crate::raw_payloads::Transport::empty()," + match[3],
    source,
)
path.write_text(source)
