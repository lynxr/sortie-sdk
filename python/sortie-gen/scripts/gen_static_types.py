"""Generate the table ``sortie.static_type`` (``catalog/static_types.toml``) from pydcs, and the
pydcs notice next to it (``catalog/THIRD-PARTY-pydcs.txt``: what the table takes from pydcs and
the license text the installed pydcs package ships).

    uv run python scripts/gen_static_types.py           # (re)write both files
    uv run python scripts/gen_static_types.py --check   # exit 1 if a committed file is stale

Both files are only ever written by this script (CI runs ``--check``).
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = (HERE / ".." / ".." / "..").resolve()
TARGET = ROOT / "catalog" / "static_types.toml"


def main(argv: list[str] | None = None) -> int:
    from sortie_gen.static_types import NOTICE_PATH, render, render_notice

    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--check", action="store_true", help="only check that the files are current")
    a = p.parse_args(argv)
    files = [(TARGET, render()), (ROOT / NOTICE_PATH, render_notice())]
    stale = []
    for path, text in files:
        current = path.read_bytes().decode("utf-8") if path.is_file() else None
        if a.check:
            if current != text:
                stale.append(path)
            continue
        path.write_bytes(text.encode("utf-8"))
        print(f"wrote {path} ({text.count(chr(10))} lines)")
    if a.check:
        for path in stale:
            print(f"{path} is stale: run scripts/gen_static_types.py", file=sys.stderr)
        if stale:
            return 1
        print(f"{TARGET} and {ROOT / NOTICE_PATH} are current")
    return 0


if __name__ == "__main__":
    sys.exit(main())
