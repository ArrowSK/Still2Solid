#!/usr/bin/env python3
"""Install Still2Solid's pinned Stable Fast 3D runtime.

This script is launched by the trusted Rust host. The Hugging Face token is
received only through the process environment and is never written to disk.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import urllib.request
from pathlib import Path

SOURCE_REVISION = "ff21fc491b4dc5314bf6734c7c0dabd86b5f5bb2"
SOURCE_BLOBS = {
    "run.py": "49d4c8780641ff0da1a310e901c0187c89a5cabf",
    "requirements.txt": "78bac25396d9f1aaf679aad00749d6582e2d6298",
    "sf3d/system.py": "1ed146cc46c19d24b3166fa7e2a27b6c79a0f9a2",
}
MODEL_REPO = "stabilityai/stable-fast-3d"
MODEL_REVISION = "f0c9a8ffd62cb1bbc8a7a53c9f87a0be1b6be778"
MODEL_SHA256 = "a3416e1cf654e7d4f5e75f116cec2c3f0a14501a77d30c2f6068bbda178de388"
DINO_REPO = "facebook/dinov2-large"
DINO_REVISION = "0ff9d1340c9524c60f3f03e8573c57a1f8197f24"
U2NET_MD5 = "60024c5c889badc19c04ad937298a77b"
TORCH_VERSION = "2.5.1"


def emit(stage: str, progress: float, overall: float, message: str) -> None:
    print(json.dumps({"stage": stage, "stageProgress": progress, "overallProgress": overall, "message": message}), flush=True)


def git_blob_sha1(path: Path) -> str:
    data = path.read_bytes()
    h = hashlib.sha1()
    h.update(f"blob {len(data)}\0".encode())
    h.update(data)
    return h.hexdigest()


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(4 * 1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def md5(path: Path) -> str:
    h = hashlib.md5(usedforsecurity=False)
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(4 * 1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def download(url: str, destination: Path, token: str | None = None) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    headers = {"User-Agent": "Still2Solid/0.8"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    temporary = destination.with_suffix(destination.suffix + ".part")
    with urllib.request.urlopen(request) as response, temporary.open("wb") as out:
        shutil.copyfileobj(response, out, length=4 * 1024 * 1024)
    temporary.replace(destination)


def safe_extract(archive: Path, destination: Path) -> None:
    destination = destination.resolve()
    with tarfile.open(archive, "r:gz") as tar:
        for member in tar.getmembers():
            target = (destination / member.name).resolve()
            if destination not in target.parents and target != destination:
                raise RuntimeError(f"Unsafe archive member: {member.name}")
        tar.extractall(destination)


def run(args: list[str], cwd: Path | None = None, env: dict[str, str] | None = None) -> None:
    subprocess.run(args, cwd=cwd, env=env, check=True)


def venv_python(root: Path) -> Path:
    if os.name == "nt":
        return root / "runtime" / "Scripts" / "python.exe"
    return root / "runtime" / "bin" / "python"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True)
    args = parser.parse_args()
    root = Path(args.root).resolve()
    token = os.environ.pop("STILL2SOLID_HF_TOKEN", "").strip()
    if not token:
        raise RuntimeError("A Hugging Face read token is required for the gated Stable Fast 3D model.")

    root.mkdir(parents=True, exist_ok=True)
    # Never allow Python tooling to write shared caches into ~/.cache or another
    # user-global location. Everything persistent for this model stays below
    # its Still2Solid model root so Model Manager can remove it completely.
    os.environ["PIP_NO_CACHE_DIR"] = "1"
    os.environ["PIP_DISABLE_PIP_VERSION_CHECK"] = "1"

    emit("source", 0.0, 0.02, "Downloading pinned Stable Fast 3D source")
    archive = root / "source.tar.gz"
    source_url = f"https://github.com/Stability-AI/stable-fast-3d/archive/{SOURCE_REVISION}.tar.gz"
    download(source_url, archive)
    unpack = root / "source-unpack"
    unpack.mkdir(exist_ok=True)
    safe_extract(archive, unpack)
    children = [p for p in unpack.iterdir() if p.is_dir()]
    if len(children) != 1:
        raise RuntimeError("Pinned Stable Fast 3D archive had an unexpected layout.")
    source = root / "source"
    shutil.move(str(children[0]), source)
    shutil.rmtree(unpack)
    archive.unlink(missing_ok=True)
    for relative, expected in SOURCE_BLOBS.items():
        actual = git_blob_sha1(source / relative)
        if actual != expected:
            raise RuntimeError(f"Stable Fast 3D source verification failed for {relative}: {actual}")
    emit("source", 1.0, 0.12, "Pinned source verified")

    emit("runtime", 0.05, 0.14, "Creating the private Stable Fast 3D runtime")
    run([sys.executable, "-m", "venv", str(root / "runtime")])
    py = venv_python(root)
    run([str(py), "-m", "pip", "install", "--disable-pip-version-check", "--no-input", "--upgrade", "pip", "wheel", "setuptools==69.5.1"])
    emit("runtime", 0.25, 0.18, f"Installing PyTorch {TORCH_VERSION}")
    run([str(py), "-m", "pip", "install", "--disable-pip-version-check", "--no-input", f"torch=={TORCH_VERSION}", "torchvision"])
    emit("runtime", 0.5, 0.23, "Installing pinned upstream dependencies")
    run([str(py), "-m", "pip", "install", "--disable-pip-version-check", "--no-input", "-r", "requirements.txt"], cwd=source)
    emit("runtime", 1.0, 0.31, "Private model runtime ready")

    model = root / "model"
    model.mkdir(exist_ok=True)
    base = f"https://huggingface.co/{MODEL_REPO}/resolve/{MODEL_REVISION}"
    emit("weights", 0.0, 0.32, "Downloading gated Stable Fast 3D configuration")
    download(f"{base}/config.yaml?download=true", model / "config.yaml", token)
    emit("weights", 0.02, 0.34, "Downloading the 4.02 GB gated Stable Fast 3D checkpoint")
    download(f"{base}/model.safetensors?download=true", model / "model.safetensors", token)
    actual = sha256(model / "model.safetensors")
    if actual != MODEL_SHA256:
        raise RuntimeError(f"Stable Fast 3D checkpoint SHA-256 mismatch: expected {MODEL_SHA256}, got {actual}")
    emit("weights", 1.0, 0.73, "Stable Fast 3D checkpoint verified")

    # Pin DINO and CLIP support assets into model-owned caches during installation
    # so inference can run offline and uninstalling the model removes them too.
    hf_home = root / "hf-cache"
    env = os.environ.copy()
    env["HF_HOME"] = str(hf_home)
    env["TORCH_HOME"] = str(root / "torch-cache")
    env["XDG_CACHE_HOME"] = str(root / "cache")
    env["HF_HUB_DISABLE_TELEMETRY"] = "1"
    emit("support", 0.05, 0.75, "Caching pinned DINO image encoder for offline inference")
    code = (
        "from huggingface_hub import snapshot_download; "
        f"print(snapshot_download('{DINO_REPO}', revision='{DINO_REVISION}', "
        "allow_patterns=['config.json','preprocessor_config.json','model.safetensors']))"
    )
    result = subprocess.run([str(py), "-c", code], env=env, check=True, capture_output=True, text=True)
    dino_path = result.stdout.strip().splitlines()[-1]
    config = (model / "config.yaml").read_text(encoding="utf-8")
    if 'facebook/dinov2-large' not in config:
        raise RuntimeError("Stable Fast 3D config no longer references the audited DINO encoder.")
    config = config.replace('"facebook/dinov2-large"', json.dumps(dino_path))
    (model / "config.yaml").write_text(config, encoding="utf-8")

    emit("support", 0.45, 0.83, "Caching CLIP material estimator for offline inference")
    clip_code = "import open_clip; open_clip.create_model_and_transforms('ViT-B-32', pretrained='laion2b_s34b_b79k'); print('ok')"
    run([str(py), "-c", clip_code], env=env)

    rembg_dir = root / "rembg"
    u2net = rembg_dir / "u2net.onnx"
    emit("support", 0.75, 0.89, "Downloading foreground-isolation support asset")
    download("https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx", u2net)
    if md5(u2net) != U2NET_MD5:
        raise RuntimeError("Foreground-isolation asset checksum mismatch.")

    manifest = {
        "schema": 1,
        "modelId": "sf3d",
        "sourceRevision": SOURCE_REVISION,
        "weightRevision": MODEL_REVISION,
        "weightSha256": MODEL_SHA256,
        "dinoRevision": DINO_REVISION,
        "pythonVersion": sys.version.split()[0],
        "licenseAccepted": True,
        "tokenStored": False,
    }
    (root / "install.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    emit("complete", 1.0, 1.0, "Stable Fast 3D runtime installed. The access token was not stored.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as exc:
        print(f"Installer command failed with exit code {exc.returncode}: {' '.join(exc.cmd)}", file=sys.stderr)
        raise
