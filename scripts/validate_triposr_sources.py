#!/usr/bin/env python3
"""Verify every TripoSR source file pinned by the production installer.

This deliberately uses the same raw GitHub URLs that the desktop installer uses. It
catches renamed/missing upstream paths and Git-blob checksum drift before a release is
built, without downloading the large model checkpoint.
"""

from __future__ import annotations

import hashlib
import re
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INSTALLER = ROOT / "src-tauri" / "src" / "triposr_installer.rs"
SOURCE = INSTALLER.read_text(encoding="utf-8")

revision_match = re.search(
    r'const TRIPOSR_SOURCE_REVISION: &str = "([0-9a-f]{40})";', SOURCE
)
manifest_match = re.search(
    r'pub\(crate\) const SOURCE_FILES: &\[\(&str, &str\)\] = &\[(.*?)\n\];',
    SOURCE,
    re.S,
)
if not revision_match or not manifest_match:
    raise SystemExit("Could not parse the TripoSR source manifest from triposr_installer.rs")

revision = revision_match.group(1)
entries = re.findall(r'\("([^"]+)", "([0-9a-f]{40})"\)', manifest_match.group(1))
if not entries:
    raise SystemExit("TripoSR source manifest is empty")

for relative, expected in entries:
    url = (
        "https://raw.githubusercontent.com/VAST-AI-Research/TripoSR/"
        f"{revision}/{relative}"
    )
    request = urllib.request.Request(url, headers={"User-Agent": "Still2Solid-CI"})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            data = response.read()
    except Exception as exc:
        raise SystemExit(f"Pinned TripoSR source is unavailable: {relative}: {exc}") from exc

    header = f"blob {len(data)}\0".encode("utf-8")
    actual = hashlib.sha1(header + data).hexdigest()
    if actual != expected:
        raise SystemExit(
            f"Pinned TripoSR source checksum mismatch for {relative}: "
            f"expected {expected}, got {actual}"
        )
    print(f"verified {relative} {actual}")

print(f"Verified {len(entries)} TripoSR source files at {revision}")
