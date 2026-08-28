# Third-party notices

This file is the checked-in development inventory for Still2Solid 0.8.0. A public signed release should still generate and review a complete SBOM and dependency-licence report from the final locked artifacts.

## Application dependencies

| Component | Purpose | Licence / status |
| --- | --- | --- |
| Tauri | Desktop application framework and packaging | MIT / Apache-2.0 |
| Svelte | UI framework | MIT |
| Three.js | 3D preview, GLB parsing, OBJ/STL export | MIT |
| fflate | Local ZIP packaging for OBJ + MTL + textures | MIT |
| Vite | Frontend build tooling | MIT |
| TypeScript | Frontend language tooling | Apache-2.0 |
| sysinfo | Native hardware information | MIT |
| reqwest | Verified model/source downloads | MIT / Apache-2.0 |
| sha1 / sha2 / md5 / base64 | Integrity and transport helpers | permissive open-source licences; release SBOM required |

## Bundled Python runtime

Packaged M7 builds prepare CPython 3.12 from `astral-sh/python-build-standalone` release artifacts pinned by filename and SHA-256 in `scripts/python-runtime.json`. The bundled interpreter is an application resource and is used as the base for model-specific private environments. CPython and the components included by python-build-standalone retain their own upstream licences. The release artifact/SBOM review is authoritative for the exact redistributed set.

## TripoSR production runtime

| Component | Purpose | Licence / status |
| --- | --- | --- |
| TripoSR source | Single-image 3D reconstruction | MIT; immutable source revision pinned |
| TripoSR pretrained checkpoint | Reconstruction weights | MIT per upstream TripoSR release; immutable revision + SHA-256 pinned |
| PyTorch | Tensor runtime / MPS / CUDA / CPU | BSD-style |
| OmegaConf | TripoSR configuration | BSD-3-Clause |
| Pillow | Image handling | HPND-style Pillow licence |
| einops | Tensor reshaping | MIT |
| Transformers | DINO ViT implementation | Apache-2.0 |
| trimesh | Mesh representation and GLB export | MIT |
| rembg | Foreground isolation | MIT |
| U-2-Net / U2Net asset | Foreground-isolation model | Apache-2.0 upstream project; downloaded asset checksum verified |
| imageio | TripoSR utility dependency | BSD-2-Clause |
| xatlas | UV atlas generation | MIT |
| ModernGL | Texture-atlas rasterization | MIT |
| scikit-image | CPU marching-cubes implementation | BSD-3-Clause |

## Stable Fast 3D production runtime

Stable Fast 3D is optional and gated. It is not covered by the Still2Solid Apache-2.0 licence.

| Component | Purpose | Licence / status |
| --- | --- | --- |
| Stable Fast 3D source | Image-to-3D reconstruction implementation | Upstream repository licence/notice applies; source pinned to commit `ff21fc491b4dc5314bf6734c7c0dabd86b5f5bb2` |
| Stable Fast 3D checkpoint | Gated reconstruction weights | Stability AI Community License; model revision `f0c9a8ffd62cb1bbc8a7a53c9f87a0be1b6be778`; SHA-256 verified |
| PyTorch / torchvision | Tensor runtime | Their upstream licences apply |
| open_clip_torch | CLIP material-estimation dependency | Upstream licence applies |
| Transformers / Hugging Face Hub | DINO/configuration and install-time asset caching | Their upstream licences apply |
| rembg / U2Net | Optional foreground isolation | MIT / Apache-2.0 upstream project as applicable |
| trimesh, Pillow, einops, OmegaConf and SF3D requirements | Mesh/image/configuration runtime | Their upstream licences apply; final release SBOM is required |
| texture_baker / uv_unwrapper and native build dependencies | UV/material pipeline supplied by SF3D source | Upstream source/dependency licences apply; final release SBOM is required |

The current Stability AI model terms are conditional. Still2Solid requires explicit user acknowledgement before installation, does not accept the upstream gate on the user's behalf, and does not persist the user's Hugging Face token.

## Weight and source handling

Still2Solid does not place third-party AI model weights in this Git repository. Model installers fetch only reviewed assets, verify pinned source/model integrity and activate installations only after verification succeeds. Generation then runs offline in one-shot child processes.

This inventory is informational and does not alter any third-party licence, acceptable-use policy, model gate or commercial term.
