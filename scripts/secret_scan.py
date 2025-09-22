#!/usr/bin/env python3
import pathlib
import re
import sys

PATTERNS = {
    "AWS Access Key": re.compile(r"AKIA[0-9A-Z]{16}"),
    "Secret Key": re.compile(r"ASIA[0-9A-Z]{16}"),
    "Private Key": re.compile(r"-----BEGIN (?:RSA|DSA|EC|PGP) PRIVATE KEY-----"),
    "Slack Token": re.compile(r"xox[baprs]-[0-9a-zA-Z-]{10,}"),
    "Generic Secret": re.compile(r"(?i)secret[_-]?key\s*=\s*[\"'][^\"']+"),
}

BINARY_EXTENSIONS = {".png", ".ico", ".jpg", ".jpeg", ".svg", ".gif", ".webp"}


def scan_file(path: pathlib.Path) -> list[str]:
    try:
        content = path.read_text(encoding="utf-8")
    except Exception:
        return []
    warnings: list[str] = []
    for name, pattern in PATTERNS.items():
        if pattern.search(content):
            warnings.append(f"{name} detected in {path}")
    return warnings


def main() -> int:
    files = [pathlib.Path(arg) for arg in sys.argv[1:]]
    if not files:
        return 0
    problems: list[str] = []
    for file_path in files:
        if not file_path.exists() or file_path.suffix.lower() in BINARY_EXTENSIONS:
            continue
        problems.extend(scan_file(file_path))
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
