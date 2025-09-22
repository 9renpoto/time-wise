#!/usr/bin/env python3
import json
import pathlib
import re
import sys

RE_WORD = re.compile(r"[A-Za-z][A-Za-z'\-]+")
DICTIONARY_PATH = pathlib.Path("/usr/share/dict/words")
CSPELL_PATH = pathlib.Path("cspell.json")

extra_words = set()
if CSPELL_PATH.exists():
    data = json.loads(CSPELL_PATH.read_text())
    extra_words.update(word.lower() for word in data.get("words", []))

dictionary = set()
if DICTIONARY_PATH.exists():
    dictionary.update(word.strip().lower() for word in DICTIONARY_PATH.read_text().splitlines())

dictionary.update(extra_words)

ALLOW_PREFIXES = {"http", "https", "file", "github"}


def is_valid_word(word: str) -> bool:
    if len(word) <= 2:
        return True
    if word.isupper():
        return True
    if any(char.isdigit() for char in word):
        return True
    lower_word = word.lower()
    if any(lower_word.startswith(prefix) for prefix in ALLOW_PREFIXES):
        return True
    if lower_word in dictionary:
        return True
    parts = re.findall(r"[A-Z]?[a-z]+|[A-Z]+(?=[A-Z]|$)", word)
    if len(parts) > 1 and all(part.lower() in dictionary or len(part) <= 2 for part in parts):
        return True
    return False


def check_file(path: pathlib.Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except Exception:
        return []
    issues = []
    for match in RE_WORD.finditer(text):
        token = match.group(0)
        if not is_valid_word(token):
            issues.append(f"Unknown word '{token}' in {path}:{match.start(0) + 1}")
    return issues


def main() -> int:
    files = [pathlib.Path(arg) for arg in sys.argv[1:]]
    if not files:
        return 0
    problems: list[str] = []
    for file_path in files:
        if not file_path.exists() or file_path.suffix in {".png", ".ico", ".svg"}:
            continue
        problems.extend(check_file(file_path))
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
