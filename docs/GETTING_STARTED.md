# Getting Started

This guide is for the **current development build** of Still2Solid. There is not yet a signed public installer. M7 is planned to package the application and its Python runtime so normal users do not need a development toolchain.

If you only want to understand the workflow, skip the build instructions and read the [User Guide](USER_GUIDE.md).

## What you need today

For the desktop application itself:

- Node.js 22 or another version compatible with the current CI configuration;
- npm;
- a stable Rust toolchain;
- the normal Tauri 2 prerequisites for your operating system.

For the current **TripoSR development runtime installer**:

- Python **3.11 or 3.12** must be discoverable on the machine when the private runtime is created.

Once Still2Solid has created its private TripoSR environment, inference runs with that environment's own Python. The user's global Python environment is not used as the inference environment.

### macOS

Install Xcode Command Line Tools if they are not already present. Apple Silicon is detected automatically. Metal/MPS is exposed as a backend choice, but availability does not mean a particular machine is guaranteed to be a good fit for every model.

### Windows and Linux

Install the platform prerequisites required by Tauri 2 and a stable Rust toolchain. NVIDIA hardware is detected through `nvidia-smi` when it is available.

## Clone and launch

```bash
git clone https://github.com/ArrowSK/Still2Solid.git
cd Still2Solid
npm install
npm run tauri:dev
```

For a quick browser-only UI preview you can use:

```bash
npm run dev
```

The browser preview cannot expose the same native hardware/runtime information as Tauri, so do not use it to make hardware-compatibility conclusions.

## Verify the checkout before changing anything

Run the same core checks used by CI:

```bash
npm run check
npm run test
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
python3 -m py_compile workers/triposr_worker.py
```

A clean checkout should pass these checks without downloading production model weights.

## First launch

1. Open **Models**.
2. Read the detected hardware summary and the compatibility reason, not just the badge.
3. If TripoSR is suitable for the machine, install it from Model Manager. On constrained hardware the button may deliberately say **Install experimental**.
4. Wait for installation and verification to complete. Do not interrupt the process while the runtime is being staged.
5. Return to the main screen and choose an image.

If TripoSR is absent or cannot be activated, Still2Solid keeps the deterministic Mock3D fallback available. That is intentional; a broken production runtime should not make the whole application unusable.

## Choosing a useful test image

For the first real generation, use a simple object with:

- most of the object visible;
- limited occlusion;
- reasonable lighting;
- a clear silhouette;
- some texture/detail on the surface;
- no need to infer a hidden back side precisely.

A busy room photo is a poor first test. If surrounding scenery is present, Still2Solid's **Background check** will normally recommend foreground isolation. You can accept or override that suggestion.

## What happens during generation

The production path is local:

1. the input is normalized locally;
2. optional foreground isolation uses the locally installed U2Net/rembg asset;
3. a one-shot TripoSR worker is launched;
4. the worker loads the pinned local model;
5. geometry and texture are generated;
6. a GLB is returned to the application;
7. the worker exits and memory can be reclaimed.

There is no persistent localhost inference service.

## After generation

The result screen gives you two distinct paths.

**Export** keeps the generated GLB as the canonical master and can derive:

- exact GLB;
- OBJ + MTL + available textures in a ZIP;
- raw geometry-only STL.

**Prepare for print** creates a separate geometry copy where you can:

- set a real target size in millimetres;
- rotate by 90° steps;
- inspect topology;
- apply conservative automatic repair;
- optionally flatten a small base band;
- export 3MF or a prepared STL.

The source photograph does not provide reliable real-world scale. Still2Solid therefore never invents millimetre dimensions from the photo.

## Current release limitation

The development build still relies on a discoverable Python 3.11/3.12 installation to create the production runtime. This is the main end-user packaging gap scheduled for **M7**.

Also, the 8 GB Apple Silicon path is intentionally not called a safe automatic recommendation yet. The code supports an explicit experimental install, but target-device benchmarking still needs to be completed before the recommendation policy can be relaxed.

## Next reading

- [User Guide](USER_GUIDE.md)
- [Models & Hardware](MODELS_AND_HARDWARE.md)
- [Troubleshooting](TROUBLESHOOTING.md)
- [Security & Privacy](SECURITY_PRIVACY.md)
