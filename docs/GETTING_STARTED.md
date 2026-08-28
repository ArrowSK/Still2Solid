# Getting Started

Still2Solid now has M1–M8 implementation in the repository. The source checkout is for development; normal release builds are designed to bundle their own pinned Python 3.12 runtime so end users are not asked to install Python, Conda or Homebrew.

A public installer should still be treated as a release only after the release workflow has produced and validated the intended platform artifact and, where applicable, signing/notarization has completed.

If you only want to understand the workflow, read the [User Guide](USER_GUIDE.md).

## Building from source

You need:

- Node.js 22 or a version compatible with the current CI configuration;
- npm;
- a stable Rust toolchain;
- the normal Tauri 2 prerequisites for your operating system.

Development model installation can use Python 3.11/3.12 through `STILL2SOLID_PYTHON` or a discoverable interpreter. Packaged builds instead prefer Still2Solid's bundled, checksum-verified Python 3.12 resource.

### macOS

Install Xcode Command Line Tools if they are not already present. Apple Silicon is detected automatically. Metal/MPS is exposed as a backend choice, but availability does not imply that every model is a good fit for every amount of unified memory.

### Windows and Linux

Install the platform prerequisites required by Tauri 2 and a stable Rust toolchain. NVIDIA hardware is detected through `nvidia-smi` when available.

## Clone and launch

```bash
git clone https://github.com/ArrowSK/Still2Solid.git
cd Still2Solid
npm ci
npm run tauri:dev
```

For a browser-only UI preview:

```bash
npm run dev
```

Browser preview cannot expose the same native hardware/runtime information as Tauri and must not be used for compatibility conclusions.

## Verify the checkout

Run the same core checks used by CI:

```bash
npm ci
npm run check
npm run test
npm run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
cargo test --locked --manifest-path src-tauri/Cargo.toml
python3 -m py_compile \
  workers/triposr/worker.py \
  workers/sf3d/install.py \
  workers/sf3d/worker.py \
  scripts/prepare_python_runtime.py
```

CI also downloads one pinned Linux Python runtime and verifies its SHA-256 and interpreter version. It deliberately does not download multi-gigabyte production model weights.

## First launch

1. Open **Models** and read the detected hardware summary and compatibility reasons.
2. Install TripoSR when it is appropriate for the machine. On constrained hardware the button may deliberately say **Install experimental**.
3. Stable Fast 3D is optional. It requires gated Hugging Face access, explicit acceptance of its model licence and a user-supplied read token. Still2Solid does not persist that token.
4. Wait for the selected model installation and verification to finish.
5. Choose an image, quality preset and optional foreground isolation, then generate.

If no verified production runtime is selected, Still2Solid keeps the deterministic Mock3D fallback available.

## Choosing a useful test image

Start with a single object that has a clear silhouette, limited occlusion, reasonable lighting and useful surface detail. A busy room photo is a poor first test. When surrounding scenery is likely present, the local **Background check** can recommend foreground isolation; the user remains in control of that setting.

## What happens during production generation

1. The image is normalized locally.
2. Optional foreground isolation runs locally.
3. Still2Solid launches a one-shot model worker.
4. The worker loads only locally installed, verified model assets.
5. Geometry and texture are generated.
6. A GLB is returned to the application.
7. The worker exits so memory can be reclaimed.

There is no persistent localhost inference service and production inference is configured for offline model access.

## After generation

**Export** keeps the generated GLB as the canonical master and can derive exact GLB, OBJ + MTL + textures in a ZIP, or raw geometry-only STL.

**Prepare for print** works on a separate geometry copy where you can set an explicit size in millimetres, orient the model, inspect/repair topology, optionally create a flat base and export 3MF or prepared STL.

Still2Solid does not infer real-world millimetre scale from a single photograph.

## Remaining validation gate

The 8 GB Apple Silicon TripoSR path remains deliberately marked memory-constrained/experimental until a real target-machine benchmark measures memory pressure, CPU/MPS behavior, quality presets, cancellation/recovery and actual end-to-end timing. This is physical validation, not missing M1–M8 implementation.

## Next reading

- [User Guide](USER_GUIDE.md)
- [Models & Hardware](MODELS_AND_HARDWARE.md)
- [M7 packaging](M7.md)
- [M8 models](M8.md)
- [Troubleshooting](TROUBLESHOOTING.md)
- [Security & Privacy](SECURITY_PRIVACY.md)
