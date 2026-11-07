#!/usr/bin/env python3
"""Run the asset-free GPU entity fixture through production source staging (requires a display)."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

import prebuilt_game


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    target = prebuilt_game.ROOT / "target" / "standard-pack-gpu-entities"
    staged = prebuilt_game.stage(target)
    crate = staged / "crates/horizon-client-render"
    shutil.copy2(crate / "fixtures/standard_pack_gpu_entities.rs",
                 crate / "examples/standard_pack_gpu_entities.rs")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="homes-", dir=target) as home:
        env = dict(os.environ, CARGO_TARGET_DIR=str(target / "build"),
                   HORIZON_LAUNCHER_NO_SELF_UPDATE="1", HORIZON_NO_AUDIO="1")
        for name in ("HORIZON_HOME", "HORIZON_LAUNCHER_HOME", "XDG_DATA_HOME", "XDG_CONFIG_HOME",
                     "HORIZON_LAUNCHER_TRASH", "HORIZON_LAUNCHER_CLIPBOARD", "HORIZON_LAUNCHER_DESKTOP"):
            path = Path(home) / name
            path.mkdir()
            env[name] = str(path)
        subprocess.run(["cargo", "run", "--profile", "dev", "-p", "horizon-client-render",
                        "--example", "standard_pack_gpu_entities", "--", str(output)],
                       cwd=staged, env=env, check=True)


if __name__ == "__main__":
    main()
