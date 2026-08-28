<p align="center">
  <img src="assets/branding/still2solid-logo.svg" alt="Still2Solid — local image to 3D" width="520">
</p>

<p align="center">
  <strong>Turn one image into a local, textured 3D asset — then preview, export or prepare it for printing.</strong>
</p>

<p align="center">
  <img alt="Version 0.8.0" src="https://img.shields.io/badge/version-0.8.0-4d8dff">
  <img alt="Apache 2.0" src="https://img.shields.io/badge/license-Apache--2.0-6c7a89">
  <img alt="Local first" src="https://img.shields.io/badge/inference-local--first-19b5a5">
  <img alt="No telemetry" src="https://img.shields.io/badge/telemetry-none-19b5a5">
  <a href="https://github.com/ArrowSK/Still2Solid/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/ArrowSK/Still2Solid/actions/workflows/ci.yml/badge.svg"></a>
</p>

Still2Solid is a desktop image-to-3D application built around a deliberately simple workflow: choose an image, let the app assess the computer, generate locally, inspect the result and export it. The application is designed to stay understandable even though the model runtimes underneath it are not simple.

> **Current state:** M1–M8 are implemented in the repository. Release packaging infrastructure is present, but a public signed/notarized binary should only be called released after the signing credentials, platform builds and target-machine checks have actually completed. The physical M1/8 GB benchmark remains an explicit validation gate.

## What it does today

- Local image selection and drag-and-drop workflow.
- Hardware-aware Model Manager and conservative compatibility policy.
- Production **TripoSR** adapter with immutable source/model revisions and checksum verification.
- Optional **Stable Fast 3D** production adapter with explicit gated access and licence acceptance; it is never silently auto-selected.
- One-shot isolated local inference workers — no localhost inference server.
- Fast / Standard / Best quality presets.
- Local background check with user-controlled foreground isolation.
- Real stage progress, cancellation and learned per-machine ETA profiles.
- Interactive Three.js preview.
- Validated GLB 2.0 canonical master.
- GLB export, OBJ + MTL + texture ZIP compatibility export and raw STL export.
- Print preparation with explicit millimetre sizing, orientation, topology checks, conservative repair, optional flat base, 3MF export and prepared STL export.
- Mock3D fallback when no verified production runtime is selected.
- Bundled, checksum-verified Python 3.12 runtime for packaged builds, with model-specific private environments layered on top.
- Tauri packaging/release workflow for Apple Silicon macOS, Windows x64 and Linux x64, including macOS signing/notarization hooks.

## The five-minute mental model

```text
Image
  ↓
Background check (local, optional isolation)
  ↓
Hardware + model decision
  ↓
Verified local one-shot inference
  ↓
Validated GLB master
  ├─ Preview
  ├─ GLB / OBJ / raw STL export
  └─ Print preparation → 3MF / prepared STL
```

The generated GLB is the fidelity-preserving master. Export and print-preparation steps derive from it; they do not rewrite the original result.

## Start here

If you are building from source, read **[Getting Started](docs/GETTING_STARTED.md)**. If you mainly want to understand the product workflow, read the **[User Guide](docs/USER_GUIDE.md)**. Developers should continue with **[Development](docs/DEVELOPMENT.md)** and **[Architecture](docs/ARCHITECTURE.md)**.

Normal packaged builds are designed to carry their own pinned Python runtime. Source/development builds may still use a compatible local Python interpreter while developing or debugging model installers.

## Documentation

| Guide | What it is for |
| --- | --- |
| [Getting Started](docs/GETTING_STARTED.md) | Build and launch the project or understand packaged-runtime expectations. |
| [User Guide](docs/USER_GUIDE.md) | Image choice, background isolation, quality, generation, preview, exports and print prep. |
| [Models & Hardware](docs/MODELS_AND_HARDWARE.md) | Why Still2Solid recommends, warns about or refuses different models. |
| [Troubleshooting](docs/TROUBLESHOOTING.md) | Common install, runtime, memory, generation, export and print-repair problems. |
| [Development](docs/DEVELOPMENT.md) | Repository layout, scripts, tests, CI and contribution workflow. |
| [Architecture](docs/ARCHITECTURE.md) | Trust boundaries, adapters, workers, canonical assets and print-prep layers. |
| [Security & Privacy](docs/SECURITY_PRIVACY.md) | What leaves the computer, what is stored locally and the security invariants. |
| [Roadmap](docs/ROADMAP.md) | Completed milestones and remaining validation/release gates. |
| [Model Licence Policy](docs/MODEL_LICENSE_POLICY.md) | Rules for model licensing, gating and catalogue inclusion. |
| [Branding](docs/BRANDING.md) | Canonical logo/icon files and their use in the app and repository. |
| [M7 notes](docs/M7.md) | Packaging, bundled runtime and release boundary. |
| [M8 notes](docs/M8.md) | Second production adapter and multi-model policy. |

## Model position

**TripoSR** remains the automatic permissive production choice where the hardware policy considers it safe. Its relatively small footprint and MIT licensing make it the baseline adapter.

**Stable Fast 3D** is the second production adapter. It is gated and uses the Stability AI Community License, so installation requires the user to provide their own Hugging Face access token and explicitly accept the applicable model terms. Still2Solid does not store that token and never auto-selects SF3D. Its Apple-Silicon MPS path remains experimental and substantially more memory-hungry than TripoSR.

**TRELLIS.2 4B** remains catalogue-only because its current upstream requirements are Linux + NVIDIA with at least 24 GB VRAM. Models with incompatible regional licensing are excluded from the official catalogue.

The 8 GB Apple Silicon TripoSR path remains marked **memory constrained / experimental** until measured on the actual target machine. Code support is not treated as proof of a safe recommendation.

## Privacy and security

Source images stay local. Generation runs locally. Still2Solid has no telemetry or analytics and does not open a localhost inference service. Network access is needed only when acquiring runtimes/models or release dependencies. Production model assets are pinned and verified before activation; inference runs in one-shot child processes with runtime model downloads blocked. Stable Fast 3D credentials are supplied only to the installer process and are not persisted by Still2Solid.

## Project status

| Milestone | Status | Delivered |
| --- | --- | --- |
| M1 | Complete | Desktop shell, simple flow, Mock3D, preview. |
| M2 | Complete | Model Manager and hardware-aware recommendations. |
| M3 | Complete | TripoSR installer/runtime/worker and production GLB. |
| M4 | Complete | Learned local timing, ETA and confidence. |
| M5 | Complete | Canonical GLB inspection and export normalization. |
| M6 | Complete | Print preparation, topology repair and 3MF. |
| M7 | Complete in code | Bundled pinned Python runtime, active Tauri bundling, native icon set and cross-platform release workflow. |
| M8 | Complete in code | Audited opt-in Stable Fast 3D adapter and multi-model generation path. |
| Target-device validation | Open | Measure the physical M1/8 GB experience before changing its conservative recommendation. |
| Signed public release | Operational gate | Requires successful platform builds plus configured signing/notarization credentials; repository code alone cannot prove this. |

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md). Keep changes small, preserve existing working behavior, add tests for policy/geometry/runtime logic, and do not weaken local-first/security boundaries for convenience.

## Licence

Still2Solid application code is licensed under **Apache-2.0**. Model weights and third-party runtime components are separately licensed and do not inherit the application licence. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and [Model Licence Policy](docs/MODEL_LICENSE_POLICY.md).
