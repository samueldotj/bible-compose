"""Write the values the editions used to set, so applying one can clear them.

Applying an edition resets every key another edition sets and this one does
not, but only where the project still holds that edition's value: anything
else is the publisher's own. An edition whose value has since changed left
projects holding a value no edition sets any more, which then read as the
publisher's own and survived every switch — the Illuminated edition's old
double frame under the Modern edition's page.

This collects, from git history, every value any edition's file has set and
no edition sets now, and writes them as layered fragments under
`crates/biblecompose-config/presets/retired/`: `settings-N.toml` and
`styles-N.toml`, one value per key per layer. Run it after changing a value
in an edition, and commit what it writes:

    python scripts/retired-presets.py
"""

import pathlib
import subprocess
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
PRESETS = "crates/biblecompose-config/presets"
OUT = ROOT / PRESETS / "retired"


def git(*args):
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True, encoding="utf-8"
    ).stdout


def leaves(table, prefix=""):
    for key, value in table.items():
        path = f"{prefix}{key}"
        if isinstance(value, dict):
            yield from leaves(value, path + ".")
        else:
            yield path, value


def kind_of(path):
    return "styles" if path.endswith(".styles.toml") else "settings"


def parse(text):
    try:
        return tomllib.loads(text)
    except tomllib.TOMLDecodeError:
        return None


def main():
    # Every version of every edition file, by path.
    paths = set(
        p for p in git("log", "--all", "--format=", "--name-only", "--", PRESETS + "/*.toml").split()
        if p.endswith(".toml") and "/retired/" not in p
    )
    current = {"settings": {}, "styles": {}}
    for f in (ROOT / PRESETS).glob("*.toml"):
        doc = parse(f.read_text(encoding="utf-8"))
        for key, value in leaves(doc or {}):
            current[kind_of(f.name)].setdefault(key, []).append(value)

    former = {"settings": {}, "styles": {}}
    for path in sorted(paths):
        kind = kind_of(path)
        for commit in git("log", "--all", "--format=%H", "--", path).split():
            try:
                text = git("show", f"{commit}:{path}")
            except subprocess.CalledProcessError:
                continue  # Deleted in this commit.
            for key, value in leaves(parse(text) or {}):
                if value in current[kind].get(key, []):
                    continue
                values = former[kind].setdefault(key, [])
                if value not in values:
                    values.append(value)

    OUT.mkdir(exist_ok=True)
    for old in OUT.glob("*.toml"):
        old.unlink()
    for kind, keys in former.items():
        depth = max((len(v) for v in keys.values()), default=0)
        for layer in range(depth):
            tables = {}
            for key, values in sorted(keys.items()):
                if layer < len(values):
                    table, _, leaf = key.rpartition(".")
                    tables.setdefault(table, []).append((leaf, values[layer]))
            lines = [
                "# Values an edition used to set and no edition sets now, written by",
                "# scripts/retired-presets.py. Applying an edition clears these too.",
            ]
            for table, items in tables.items():
                lines.append("")
                if table:
                    lines.append(f"[{table}]")
                for leaf, value in items:
                    lines.append(f"{leaf} = {literal(value)}")
            (OUT / f"{kind}-{layer + 1}.toml").write_text("\n".join(lines) + "\n", encoding="utf-8")
            print(f"{kind}-{layer + 1}.toml: {sum(len(i) for i in tables.values())} values")


def literal(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return repr(value)
    if isinstance(value, str):
        escaped = value.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{escaped}"'
    if isinstance(value, list):
        return "[" + ", ".join(literal(v) for v in value) + "]"
    sys.exit(f"a value this script cannot write: {value!r}")


if __name__ == "__main__":
    main()
