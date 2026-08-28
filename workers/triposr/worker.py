#!/usr/bin/env python3
"""Pinned local TripoSR worker for Still2Solid M3.

The worker communicates only over stdin/process arguments and stdout JSON lines. It does
not expose an HTTP server and it blocks Hugging Face model downloads during inference.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import types
from pathlib import Path

import numpy as np
from PIL import Image

STARTED = time.monotonic()


def emit(stage_id: str, stage_name: str, stage_progress: float, overall_progress: float, message: str) -> None:
    payload = {
        "stageId": stage_id,
        "stageName": stage_name,
        "stageProgress": max(0.0, min(1.0, stage_progress)),
        "overallProgress": max(0.0, min(1.0, overall_progress)),
        "progressIsEstimated": True,
        "elapsedSeconds": time.monotonic() - STARTED,
        "etaSeconds": 0.0,
        "etaConfidence": "low",
        "statusMessage": message,
    }
    print(json.dumps(payload), flush=True)


def install_torchmcubes_cpu_shim() -> None:
    """Avoid the upstream CUDA/C++ torchmcubes build on Apple Silicon.

    TripoSR calls torchmcubes.marching_cubes through a tiny interface. For M3 we provide
    that interface with scikit-image on CPU, then return tensors on the source device.
    """
    import torch
    from skimage import measure

    module = types.ModuleType("torchmcubes")

    def marching_cubes(volume, threshold):
        source_device = volume.device
        field = volume.detach().float().cpu().numpy()
        vertices, faces, _normals, _values = measure.marching_cubes(field, level=float(threshold))
        vertex_tensor = torch.from_numpy(np.ascontiguousarray(vertices, dtype=np.float32)).to(source_device)
        face_tensor = torch.from_numpy(np.ascontiguousarray(faces, dtype=np.int64)).to(source_device)
        return vertex_tensor, face_tensor

    module.marching_cubes = marching_cubes
    sys.modules["torchmcubes"] = module


def block_remote_huggingface_downloads(dino_config_path: Path) -> None:
    import huggingface_hub

    def local_only_hf_hub_download(*args, **kwargs):
        repo_id = kwargs.get("repo_id") or (args[0] if args else None)
        filename = kwargs.get("filename") or (args[1] if len(args) > 1 else None)
        if repo_id == "facebook/dino-vitb16" and filename == "config.json":
            return str(dino_config_path)
        raise RuntimeError(
            f"Still2Solid blocked an unexpected Hugging Face runtime download: {repo_id}/{filename}"
        )

    huggingface_hub.hf_hub_download = local_only_hf_hub_download


def choose_device(requested: str) -> str:
    import torch

    if requested == "cpu":
        return "cpu"
    if requested == "cuda":
        if torch.cuda.is_available():
            return "cuda:0"
        raise RuntimeError("CUDA was requested but is not available in the TripoSR runtime.")
    if requested == "metal":
        if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
            return "mps"
        raise RuntimeError("Metal / MPS was requested but is not available in the TripoSR runtime.")

    if torch.cuda.is_available():
        return "cuda:0"
    if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        return "mps"
    return "cpu"


def preset(quality: str) -> tuple[int, int, int]:
    # Conservative extraction settings keep unified-memory pressure below upstream defaults.
    if quality == "fast":
        return 128, 512, 512
    if quality == "best":
        return 192, 1024, 1536
    return 160, 768, 1024


def prepare_image(path: Path, remove_background: bool, rembg_home: Path) -> tuple[Image.Image, str | None]:
    image = Image.open(path).convert("RGBA")
    warning = None

    if remove_background:
        try:
            import rembg
            session = rembg.new_session("u2net")
            image = rembg.remove(image, session=session).convert("RGBA")
        except Exception as exc:  # best-effort fallback is safer than discarding a generation
            warning = f"Foreground isolation failed; the original image was used: {exc}"

    if image.mode == "RGBA" and image.getextrema()[3] != (255, 255):
        alpha = np.asarray(image.getchannel("A"), dtype=np.float32) / 255.0
        rgb = np.asarray(image.convert("RGB"), dtype=np.float32) / 255.0
        composed = rgb * alpha[..., None] + 0.5 * (1.0 - alpha[..., None])
        image = Image.fromarray(np.clip(composed * 255.0, 0, 255).astype(np.uint8), mode="RGB")
    else:
        image = image.convert("RGB")

    return image, warning


def bake_texture_glb(mesh, model, scene_code, texture_resolution: int, output_path: Path) -> None:
    import torch
    import trimesh
    from tsr.bake_texture import make_atlas, rasterize_position_atlas

    texture_padding = round(max(2, texture_resolution / 256))
    atlas = make_atlas(mesh, texture_resolution, texture_padding)
    position_atlas = rasterize_position_atlas(
        mesh,
        atlas["vmapping"],
        atlas["indices"],
        atlas["uvs"],
        texture_resolution,
        texture_padding,
    )

    flat = position_atlas.reshape(-1, 4)
    occupied = flat[:, 3] > 0.0
    rgba = np.zeros((flat.shape[0], 4), dtype=np.float32)

    if np.any(occupied):
        positions = torch.from_numpy(np.ascontiguousarray(flat[occupied, :3], dtype=np.float32)).to(scene_code.device)
        with torch.no_grad():
            colors = model.renderer.query_triplane(model.decoder, positions, scene_code)["color"]
        rgba[occupied, :3] = colors.detach().float().cpu().numpy()
        rgba[occupied, 3] = 1.0

    texture = Image.fromarray(
        np.clip(rgba.reshape(texture_resolution, texture_resolution, 4) * 255.0, 0, 255).astype(np.uint8),
        mode="RGBA",
    ).transpose(Image.Transpose.FLIP_TOP_BOTTOM)

    vertices = np.asarray(mesh.vertices[atlas["vmapping"]], dtype=np.float32)
    faces = np.asarray(atlas["indices"], dtype=np.int64)
    uvs = np.asarray(atlas["uvs"], dtype=np.float32)
    visual = trimesh.visual.texture.TextureVisuals(uv=uvs, image=texture)
    textured_mesh = trimesh.Trimesh(
        vertices=vertices,
        faces=faces,
        vertex_normals=np.asarray(mesh.vertex_normals[atlas["vmapping"]], dtype=np.float32),
        visual=visual,
        process=False,
    )
    textured_mesh.export(output_path, file_type="glb")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", required=True)
    parser.add_argument("--model-root", required=True)
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--result-json", required=True)
    parser.add_argument("--quality", choices=["fast", "standard", "best"], default="standard")
    parser.add_argument("--backend", choices=["auto", "metal", "cuda", "cpu"], default="auto")
    parser.add_argument("--remove-background", action="store_true")
    args = parser.parse_args()

    source_root = Path(args.source_root).resolve()
    model_root = Path(args.model_root).resolve()
    input_path = Path(args.input).resolve()
    output_path = Path(args.output).resolve()
    result_path = Path(args.result_json).resolve()

    os.environ.setdefault("PYTORCH_ENABLE_MPS_FALLBACK", "1")
    os.environ["U2NET_HOME"] = str(model_root / "rembg")
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"

    sys.path.insert(0, str(source_root))
    install_torchmcubes_cpu_shim()
    block_remote_huggingface_downloads(model_root / "dino-config.json")

    import torch
    from tsr.system import TSR

    mc_resolution, chunk_size, texture_resolution = preset(args.quality)
    device = choose_device(args.backend)

    emit("prepare", "Preparing image", 0.1, 0.02, "Reading the source image locally")
    image, warning = prepare_image(input_path, args.remove_background, model_root / "rembg")
    emit("isolate", "Isolating object", 1.0, 0.12, warning or "Foreground preparation complete")

    emit("load", "Loading model", 0.1, 0.15, f"Loading pinned TripoSR weights on {device}")
    model = TSR.from_pretrained(str(model_root), config_name="config.yaml", weight_name="model.ckpt")
    model.renderer.set_chunk_size(chunk_size)
    model.to(device)
    model.eval()
    emit("load", "Loading model", 1.0, 0.27, f"TripoSR ready on {device}")

    emit("reconstruct", "Reconstructing geometry", 0.05, 0.30, "Running the TripoSR reconstruction network")
    with torch.inference_mode():
        scene_codes = model([image], device=device)
    emit("reconstruct", "Reconstructing geometry", 1.0, 0.58, "Latent 3D representation complete")

    emit("mesh", "Extracting mesh", 0.1, 0.61, f"Extracting a {mc_resolution}³ surface")
    meshes = model.extract_mesh(scene_codes, True, resolution=mc_resolution)
    mesh = meshes[0]
    triangles = int(len(mesh.faces))
    emit("mesh", "Extracting mesh", 1.0, 0.76, f"Extracted {triangles:,} triangles")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    textured = False
    texture_warning = None
    try:
        emit("texture", "Baking texture", 0.1, 0.79, f"Baking a {texture_resolution}px UV texture")
        bake_texture_glb(mesh, model, scene_codes[0], texture_resolution, output_path)
        textured = True
        emit("texture", "Baking texture", 1.0, 0.94, "UV texture embedded in GLB")
    except Exception as exc:
        texture_warning = f"UV baking failed; exported vertex-colour GLB instead: {exc}"
        mesh.export(output_path, file_type="glb")
        emit("texture", "Baking texture", 1.0, 0.94, texture_warning)

    emit("preview", "Preparing preview", 0.5, 0.97, "Finalising the local GLB")
    result = {
        "triangles": triangles,
        "textured": textured,
        "backend": device,
        "mcResolution": mc_resolution,
        "textureResolution": texture_resolution if textured else 0,
        "warning": texture_warning or warning,
    }
    result_path.write_text(json.dumps(result), encoding="utf-8")
    emit("preview", "Preparing preview", 1.0, 1.0, "Production GLB ready")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(130)
    except Exception as exc:
        print(f"Still2Solid TripoSR worker failed: {exc}", file=sys.stderr, flush=True)
        raise
