"""Print how many crates a Dyna runner builds at once: its CPU quota, bounded by its memory.

The runner's cgroup (v2) holds its CPU quota and memory limit. Each concurrent rustc may need
`--job-mib` of memory. Four stays the floor: the fixed cap these runners used before.

`--cap` applies the answer to the whole job, so no build or test inside it runs at the host's
size (`nproc` there reports every core of the host, not the runner's quota): it writes Cargo's
`build.jobs` into a config above the checkout, which every Cargo run under the checkout reads,
scripts' included, and exports `RUST_TEST_THREADS`, `CI_JOBS` and `CI_CPUS` for test harnesses.
A config file rather than `CARGO_BUILD_JOBS`: sccache hashes `CARGO_*` variables, and a job count
there would split the runners' compiler cache from the boxes'.
"""
import argparse
import os
import sys
from pathlib import Path

FLOOR = 4


def limits(cgroup: Path) -> tuple[int, int]:
    """The runner's CPUs and memory bytes; an unlimited value falls back to the machine's."""
    quota, period = (cgroup / "cpu.max").read_text().split()
    cpus = len(os.sched_getaffinity(0)) if quota == "max" else max(1, int(quota) // int(period))
    memory = (cgroup / "memory.max").read_text().strip()
    total = os.sysconf("SC_PHYS_PAGES") * os.sysconf("SC_PAGE_SIZE") if memory == "max" else int(memory)
    return cpus, total


def jobs(cpus: int, memory: int, job_mib: int) -> int:
    return max(FLOOR, min(cpus, memory // (job_mib << 20)))


def cap(chosen: int, cpus: int, workspace: Path, env_file: Path) -> None:
    """Make `chosen` every Cargo run's job count under `workspace`, and `cpus` its test threads."""
    config = workspace.parent / ".cargo/config.toml"
    config.parent.mkdir(parents=True, exist_ok=True)
    config.write_text(f"# Written by tools/ci/cargo_jobs.py --cap for this job only.\n[build]\njobs = {chosen}\n")
    with env_file.open("a") as env:
        env.write(f"RUST_TEST_THREADS={cpus}\nCI_JOBS={chosen}\nCI_CPUS={cpus}\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--job-mib", type=int, required=True, help="memory one concurrent rustc may need")
    parser.add_argument("--cgroup", type=Path, default=Path("/sys/fs/cgroup"))
    parser.add_argument("--cap", action="store_true", help="apply the answer to every build and test of this job")
    args = parser.parse_args()
    cpus, memory = limits(args.cgroup)
    chosen = jobs(cpus, memory, args.job_mib)
    if args.cap:
        cap(chosen, cpus, Path(os.environ["GITHUB_WORKSPACE"]), Path(os.environ["GITHUB_ENV"]))
    print(f"runner cpus={cpus} memory_mib={memory >> 20} job_mib={args.job_mib} jobs={chosen}", file=sys.stderr)
    print(chosen)
