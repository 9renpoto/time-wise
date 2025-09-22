#!/usr/bin/env python3
import pathlib
import sys
import tomllib

SECTIONS = [
    "dependencies",
    "dev-dependencies",
    "build-dependencies",
]


def check_sections(path: pathlib.Path) -> list[str]:
    data = tomllib.loads(path.read_text())
    issues: list[str] = []
    for section in SECTIONS:
        table = data.get(section) or {}
        if not isinstance(table, dict) or len(table) <= 1:
            continue
        keys = list(table.keys())
        if keys != sorted(keys):
            issues.append(f"{path}:{section} is not sorted: {keys} != {sorted(keys)}")
    return issues


def main() -> int:
    files = [pathlib.Path("Cargo.toml"), pathlib.Path("src-tauri/Cargo.toml")]
    issues: list[str] = []
    for file_path in files:
        if file_path.exists():
            issues.extend(check_sections(file_path))
    if issues:
        print("\n".join(issues), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
