#!/usr/bin/env python3
"""Prepare Still2Solid's pinned standalone Python resource for a release build.

Downloads one immutable python-build-standalone archive, verifies SHA-256 before
extracting, and places the archive's `python/` tree under src-tauri/resources.
No downloaded interpreter is committed to Git.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import tarfile
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "scripts" / "python-runtime.json"
OUTPUT = ROOT / "src-tauri" / "resources" / "python"
BASE_URL = "https://github.com/astral-sh/python-build-standalone/releases/download/{release}/{filename}"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(4 * 1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def safe_extract(archive: Path, destination: Path) -> None:
    destination = destination.resolve()
    with tarfile.open(archive, "r:gz") as tar:
        for member in tar.getmembers():
            target = (destination / member.name).resolve()
            if destination not in target.parents and target != destination:
                raise RuntimeError(f"Unsafe archive member: {member.name}")
        tar.extractall(destination)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True, help="Rust target triple from python-runtime.json")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    spec = manifest["archives"].get(args.target)
    if spec is None:
        raise SystemExit(f"No bundled Python archive is pinned for {args.target}")

    url = BASE_URL.format(release=manifest["release"], filename=spec["filename"])
    with tempfile.TemporaryDirectory(prefix="still2solid-python-") as temp_name:
        temp = Path(temp_name)
        archive = temp / spec["filename"]
        print(f"Downloading pinned Python {manifest['version']} for {args.target}")
        with urllib.request.urlopen(url) as response, archive.open("wb") as out:
            shutil.copyfileobj(response, out)
        actual = sha256(archive)
        if actual != spec["sha256"]:
            raise SystemExit(f"Python runtime checksum mismatch: expected {spec['sha256']}, got {actual}")

        unpack = temp / "unpack"
        unpack.mkdir()
        safe_extract(archive, unpack)
        source = unpack / "python"
        if not source.is_dir():
            raise SystemExit("Pinned archive did not contain the expected python/ directory")

        output = args.output.resolve()
        if output.exists():
            shutil.rmtree(output)
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(source, output, symlinks=True)

    marker = args.output / "STILL2SOLID_RUNTIME.json"
    marker.write_text(json.dumps({
        "python": manifest["version"],
        "release": manifest["release"],
        "target": args.target,
        "source": manifest["provider"],
        "archive_sha256": spec["sha256"],
    }, indent=2) + "\n", encoding="utf-8")
    print(f"Prepared verified runtime at {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
