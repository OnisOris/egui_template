#!/usr/bin/env python3
from pathlib import Path
import sys

roots = (Path("apps"), Path("crates"))
files = sorted(path for root in roots for path in root.rglob("*.rs"))
failed = False
for path in files:
    count = len(path.read_text(encoding="utf-8").splitlines())
    if count > 300:
        print(f"review size ({count}/400): {path}")
    if count > 400:
        print(f"ERROR hard limit exceeded ({count}/400): {path}")
        failed = True
if not files:
    print("ERROR: no Rust source files found")
    failed = True
sys.exit(1 if failed else 0)
