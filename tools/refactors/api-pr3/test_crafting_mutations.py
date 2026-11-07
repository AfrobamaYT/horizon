#!/usr/bin/env python3
"""Run only after other builds finish: mutate the converter, test the real hook,
restore exact bytes, then rerun the baseline. Does not run workspace tests.
"""
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
SOURCE = ROOT / "crates/horizon-client/src/json_nbt.rs"
COMMAND = ["cargo", "test", "-p", "horizon-client-lib", "--lib",
           "custom_ingredient_tags_reach_hook_and_matching_record"]


def main():
    subprocess.run(COMMAND, cwd=ROOT, check=True)
    original = SOURCE.read_text()
    mutations = [
        ("int width", "return Tag::Int(value);", "return Tag::Short(value as i16);"),
        ("long width", "return Tag::Long(value);", "return Tag::Int(value as i32);"),
        ("list kind", "Value::Array(values) => Tag::List(", "Value::Array(values) if values.iter().all(Value::is_number) => Tag::ByteArray(vec![]),\n        Value::Array(values) => Tag::List("),
        ("nested compound", "Value::Object(values) => Tag::Compound(", "Value::Object(_) if nested => Tag::compound([] as [(&str, Tag); 0]),\n        Value::Object(values) => Tag::Compound("),
    ]
    try:
        for name, old, new in mutations:
            assert original.count(old) == 1, name
            SOURCE.write_text(original.replace(old, new))
            result = subprocess.run(COMMAND, cwd=ROOT, capture_output=True, text=True)
            if result.returncode != 101 or "test result: FAILED" not in result.stdout:
                raise AssertionError(f"Mutation survived or failed to compile: {name}\n{result.stdout}\n{result.stderr}")
            print(f"Killed by the real ingredient hook test: {name}", flush=True)
    finally:
        SOURCE.write_text(original)
    subprocess.run(COMMAND, cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
