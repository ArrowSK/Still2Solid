#!/usr/bin/env python3
"""One-shot Stable Fast 3D worker for Still2Solid.

The Rust host supplies only locally installed, audited source/model paths. Network
access is disabled by environment variables before this process is launched.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from contextlib import nullcontext
from pathlib import Path


def emit(stage_id: str, stage_name: str, stage_progress: float, overall: float, message: str, started: float) -> None:
    print(json.dumps({
        "stageId": stage_id,
        "stageName": stage_name,
        "stageProgress": max(0.0, min(1.0, stage_progress)),
        "overallProgress": max(0.0, min(1.0, overall)),
        "progressIsEstimated": True,
        "elapsedSeconds": time.monotonic() - started,
        "etaSeconds": 0.0,
        "etaConfidence": "low",
        "statusMessage": message,
    }), flush=True)


def choose_device(requested: str, torch) -> str:
    if requested == "cpu":
        return "cpu"
    if requested in {"metal", "mps"}:
        if not torch.backends.mps.is_available():
            raise RuntimeError("Metal/MPS was requested but is not available in this runtime.")
        return "mps"
    if requested == "cuda":
        if not torch.cuda.is_available():
            raise RuntimeError("CUDA was requested but is not available in this runtime.")
        return "cuda"
    if torch.cuda.is_available():
        return "cuda"
    if torch.backends.mps.is_available():
        return "mps"
    return "cpu"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", required=True)
    parser.add_argument("--model-root", required=True)
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--result-json", required=True)
    parser.add_argument("--quality", choices=["fast", "standard", "best"], required=True)
    parser.add_argument("--backend", choices=["auto", "metal", "cuda", "cpu"], required=True)
    parser.add_argument("--remove-background", action="store_true")
    args = parser.parse_args()

    started = time.monotonic()
    source_root = Path(args.source_root).resolve()
    model_root = Path(args.model_root).resolve()
    input_path = Path(args.input).resolve()
    output_path = Path(args.output).resolve()
    result_path = Path(args.result_json).resolve()
    local_model = model_root / "model"

    if not (source_root / "sf3d" / "system.py").is_file():
        raise RuntimeError("Verified Stable Fast 3D source is missing.")
    if not (local_model / "config.yaml").is_file() or not (local_model / "model.safetensors").is_file():
        raise RuntimeError("Verified Stable Fast 3D model files are missing.")

    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"
    os.environ["PYTORCH_ENABLE_MPS_FALLBACK"] = "1"
    os.environ["HF_HOME"] = str(model_root / "hf-cache")
    os.environ["U2NET_HOME"] = str(model_root / "rembg")
    sys.path.insert(0, str(source_root))

    emit("prepare", "Preparing image", 0.05, 0.02, "Opening the local source image", started)
    import numpy as np
    import torch
    from PIL import Image
    import sf3d.utils as sf3d_utils
    from sf3d.system import SF3D

    image = Image.open(input_path).convert("RGBA")
    if args.remove_background:
        import rembg
        emit("prepare", "Preparing image", 0.45, 0.08, "Removing the background locally", started)
        image = rembg.remove(image, session=rembg.new_session())
    image = sf3d_utils.resize_foreground(image, 0.85, out_size=(512, 512))
    emit("prepare", "Preparing image", 1.0, 0.12, "Image prepared", started)

    device = choose_device(args.backend, torch)
    emit("load", "Loading model", 0.05, 0.14, f"Loading Stable Fast 3D on {device}", started)
    model = SF3D.from_pretrained(str(local_model), config_name="config.yaml", weight_name="model.safetensors")
    model.eval()
    model = model.to(device)
    emit("load", "Loading model", 1.0, 0.31, "Model loaded into the selected backend", started)

    cond_width = cond_height = 512
    cond_distance = 1.6
    cond_fovy_deg = 40
    background_color = [0.5, 0.5, 0.5]
    c2w_cond = sf3d_utils.default_cond_c2w(cond_distance)
    intrinsic, intrinsic_normed_cond = sf3d_utils.create_intrinsic_from_fov_deg(cond_fovy_deg, cond_height, cond_width)
    img_cond = (
        torch.from_numpy(np.asarray(image.resize((cond_width, cond_height))).astype(np.float32) / 255.0)
        .float().clip(0, 1)
    )
    mask_cond = img_cond[:, :, -1:]
    rgb_cond = torch.lerp(torch.tensor(background_color)[None, None, :], img_cond[:, :, :3], mask_cond)
    batch_elem = {
        "rgb_cond": rgb_cond,
        "mask_cond": mask_cond,
        "c2w_cond": c2w_cond.unsqueeze(0),
        "intrinsic_cond": intrinsic.unsqueeze(0),
        "intrinsic_normed_cond": intrinsic_normed_cond.unsqueeze(0),
    }
    batch = {key: value.unsqueeze(0).to(device) for key, value in batch_elem.items()}

    texture_resolution = {"fast": 512, "standard": 1024, "best": 1536}[args.quality]
    emit("infer", "Reconstructing 3D", 0.02, 0.33, "Running local single-image reconstruction", started)
    with torch.no_grad():
        context = torch.autocast(device_type=device, dtype=torch.bfloat16) if device == "cuda" else nullcontext()
        with context:
            meshes, _ = model.generate_mesh(batch, texture_resolution, "none", -1)
    mesh = meshes[0] if isinstance(meshes, (list, tuple)) else meshes
    emit("infer", "Reconstructing 3D", 1.0, 0.90, "Mesh and material reconstruction complete", started)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    emit("export", "Building GLB", 0.2, 0.92, "Packing the canonical GLB", started)
    mesh.export(str(output_path), file_type="glb", include_normals=True)
    faces = getattr(mesh, "faces", None)
    triangles = int(len(faces)) if faces is not None else 0
    result = {
        "triangles": triangles,
        "textured": True,
        "backend": device,
        "mcResolution": 0,
        "textureResolution": texture_resolution,
        "warning": "Stable Fast 3D is an optional gated model under the Stability AI Community License; use remains subject to those terms."
    }
    result_path.write_text(json.dumps(result), encoding="utf-8")
    emit("export", "Building GLB", 1.0, 1.0, "Canonical GLB ready", started)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"Still2Solid SF3D worker failed: {exc}", file=sys.stderr, flush=True)
        raise
