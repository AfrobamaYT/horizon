"""Own the mutable datafix root, independently of read-only vanilla inputs."""
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VARIABLE = 'HORIZON_DATAFIX_ROOT'
READ_ONLY = Path('/box/shared/vanilla')


def writable_root():
    """Resolve and reject shared fixture destinations before creating anything."""
    configured = os.environ.get(VARIABLE)
    root = Path(configured).expanduser().resolve() if configured else (ROOT / '.local' / 'datafix').resolve()
    forbidden = READ_ONLY.resolve()
    if root.is_relative_to(forbidden) or (not configured and not root.is_relative_to(ROOT.resolve())):
        raise ValueError(f'{VARIABLE} writable root resolves to {root}; shared fixture inputs are read-only. '
                         f'Set {VARIABLE} to a private directory (and remove any escaping .local symlink).')
    return root
