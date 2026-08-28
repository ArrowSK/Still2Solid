# Third-party notices

This is the development inventory for Still2Solid M5. Signed releases must generate a complete SBOM and dependency-licence report from the final locked artifacts.

## Application dependencies

| Component | Purpose | Licence |
| --- | --- | --- |
| Tauri | Desktop application framework | MIT / Apache-2.0 |
| Svelte | UI framework | MIT |
| Three.js | 3D preview, GLB parsing, OBJ/STL export | MIT |
| fflate | Local ZIP packaging for OBJ + MTL + textures | MIT |
| Vite | Frontend build tooling | MIT |
| TypeScript | Frontend language tooling | Apache-2.0 |
| sysinfo | Native hardware information | MIT |
| reqwest | Verified model/source downloads | MIT / Apache-2.0 |
| sha1 / sha2 / md5 / base64 | Integrity and transport helpers | permissive open-source licences; release SBOM required |

## TripoSR production runtime

| Component | Purpose | Licence / status |
| --- | --- | --- |
| TripoSR source | Single-image 3D reconstruction | MIT; source revision pinned by M3 |
| TripoSR pretrained checkpoint | Reconstruction weights | MIT per upstream TripoSR release; revision + SHA-256 pinned by M3 |
| PyTorch 2.13.0 | Tensor runtime / MPS / CUDA / CPU | BSD-style |
| OmegaConf 2.3.0 | TripoSR configuration | BSD-3-Clause |
| Pillow 12.1.0 | Image handling | HPND-style Pillow licence |
| einops 0.8.1 | Tensor reshaping | MIT |
| Transformers 4.35.0 | DINO ViT implementation | Apache-2.0 |
| trimesh 4.0.5 | Mesh representation and GLB export | MIT |
| rembg 2.0.77 | Foreground isolation | MIT |
| U-2-Net / U2Net asset | Foreground-isolation model | Apache-2.0 upstream project; downloaded asset checksum verified against rembg |
| imageio 2.37.0 | TripoSR utility dependency | BSD-2-Clause |
| xatlas 0.0.11 | UV atlas generation | MIT |
| ModernGL 5.10.0 | Texture-atlas rasterization | MIT |
| scikit-image 0.26.0 | CPU marching-cubes implementation | BSD-3-Clause |

Still2Solid does not place third-party AI weights in this Git repository. The installer downloads only the hard-coded TripoSR/U2Net assets described in `docs/M3.md`, verifies them, and activates them only after verification succeeds.

M5 adds no new AI model or model-weight licence. OBJ/STL conversion uses Three.js exporters already covered by the Three.js MIT licence, and ZIP packaging uses fflate under MIT.

This inventory is informational and does not alter any third-party licence terms.
