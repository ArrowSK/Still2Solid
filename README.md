<p align="center">
  <img src="assets/branding/still2solid-logo.webp" alt="Still2Solid — local image to 3D" width="520">
</p>

<p align="center">
  <strong>Turn one image into a local, textured 3D asset — then preview, export or prepare it for printing.</strong>
</p>

<p align="center">
  <img alt="Version 0.6.1" src="https://img.shields.io/badge/version-0.6.1-4d8dff">
  <img alt="Apache 2.0" src="https://img.shields.io/badge/license-Apache--2.0-6c7a89">
  <img alt="Local first" src="https://img.shields.io/badge/inference-local--first-19b5a5">
  <img alt="No telemetry" src="https://img.shields.io/badge/telemetry-none-19b5a5">
  <a href="https://github.com/ArrowSK/Still2Solid/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/ArrowSK/Still2Solid/actions/workflows/ci.yml/badge.svg"></a>
</p>

Still2Solid is a desktop image-to-3D application built around a deliberately simple workflow: choose an image, let the app pick a sensible local model for the computer, generate, inspect the result and export it. The application is designed to stay understandable even though the model runtime underneath it is not simple.

> **Current state:** M1–M6 are implemented. The repository is a working development build, not yet a signed/notarized end-user release. Release packaging and the bundled runtime are planned for M7.

## What it does today

- Local image selection and drag-and-drop workflow.
- Hardware-aware model assessment and Model Manager.
- Production TripoSR adapter with pinned source/model revisions and checksum verification.
- One-shot isolated local inference worker — no localhost inference server.
- Fast / Standard / Best quality presets.
- Local background check that suggests foreground isolation when surrounding scenery is likely present; the user always remains in control of the setting.
- Real stage progress, cancellation and learned per-machine ETA profiles.
- Interactive Three.js preview.
- Validated GLB 2.0 canonical master.
- Exact GLB export, OBJ + MTL + texture ZIP compatibility export and raw STL export.
- Print preparation with explicit millimetre sizing, orientation, topology checks, conservative repair, optional flat base, 3MF export and prepared STL export.
- Mock3D fallback when the production runtime is absent, unsupported or intentionally not selected.

## The five-minute mental model

```text
Image
  ↓
Background check (local, optional removal)
  ↓
Hardware/model decision
  ↓
Local one-shot inference
  ↓
Validated GLB master
  ├─ Preview
  ├─ GLB / OBJ / raw STL export
  └─ Print preparation → 3MF / prepared STL
```

The generated GLB is the fidelity-preserving master. Export and print-preparation steps derive from it; they do not rewrite the original result.

## Start here

If you are trying the app from source, read **[Getting Started](docs/GETTING_STARTED.md)** first. If you mainly want to understand how to use it, read the **[User Guide](docs/USER_GUIDE.md)**. Developers should continue with **[Development](docs/DEVELOPMENT.md)** and **[Architecture](docs/ARCHITECTURE.md)**.

The current development build needs Node.js, Rust/Tauri prerequisites and — for installing the TripoSR runtime — Python 3.11 or 3.12. M7 is intended to remove that Python prerequisite from the normal end-user experience by bundling the runtime.

## Documentation

| Guide | What it is for |
| --- | --- |
| [Getting Started](docs/GETTING_STARTED.md) | Build and launch the current development version without guessing the steps. |
| [User Guide](docs/USER_GUIDE.md) | Image choice, background removal, quality, generation, preview, exports and print prep. |
| [Models & Hardware](docs/MODELS_AND_HARDWARE.md) | Why Still2Solid recommends some models and refuses or warns about others. |
| [Troubleshooting](docs/TROUBLESHOOTING.md) | Common install, runtime, memory, generation, export and print-repair problems. |
| [Development](docs/DEVELOPMENT.md) | Repository layout, scripts, tests, CI and contribution workflow. |
| [Architecture](docs/ARCHITECTURE.md) | Trust boundaries, adapters, workers, canonical assets and print-prep layers. |
| [Security & Privacy](docs/SECURITY_PRIVACY.md) | What leaves the computer, what is stored locally and the security invariants. |
| [Roadmap](docs/ROADMAP.md) | What is complete, what still needs validation and what M7/M8 are intended to deliver. |
| [Model Licence Policy](docs/MODEL_LICENSE_POLICY.md) | Rules for model licensing, gating and catalogue inclusion. |
| [Branding](docs/BRANDING.md) | Logo/icon files and how they should be used. |
| [Milestone notes](docs/M1.md) | Historical implementation notes from M1 onward. |

## Current model position

**TripoSR** is the first production adapter because it is comparatively lightweight and permissively licensed. Still2Solid does not claim that every backend or computer is equally suitable. In particular, the 8 GB Apple Silicon path remains deliberately marked **memory constrained / experimental** until it is benchmarked on the actual target hardware.

Other catalogue entries are informational unless their hardware and licence requirements clear Still2Solid's policy. Conditional/gated models are never silently auto-selected. Models with incompatible regional licensing are excluded from the official catalogue.

## Privacy and security in one paragraph

Source images stay local. Generation runs locally. Still2Solid has no telemetry or analytics and does not open a localhost inference service. Network access is needed when installing the production runtime/model; source and model assets are pinned and verified before activation. Inference runs in a one-shot child process and runtime model downloads are blocked during generation. See [Security & Privacy](docs/SECURITY_PRIVACY.md) for the full boundary.

## Project status and roadmap

| Milestone | Status | Delivered / planned |
| --- | --- | --- |
| M1 | ✅ Complete | Desktop shell, simple flow, Mock3D, preview. |
| M2 | ✅ Complete | Model Manager and hardware-aware recommendations. |
| M3 | ✅ Complete | Real TripoSR installer/runtime/worker and production GLB. |
| M4 | ✅ Complete | Learned local timing, ETA and confidence. |
| M5 | ✅ Complete | Canonical GLB inspection and export normalization. |
| M6 | ✅ Complete | Print preparation, topology repair and 3MF. |
| 0.6.1 polish | ✅ Complete | Product branding, humane documentation and local background-removal guidance. |
| Target-device validation | ⏳ Open | Measure the real M1/8 GB experience before calling it a safe recommendation. |
| M7 | 🧭 Planned | End-user packaging, bundled runtime, signing/notarization and release workflow. |
| M8 | 🧭 Planned | Additional models only after hardware, licence and maintenance review. |

The roadmap is intentionally conservative: a capability is not considered safe merely because the code path exists.

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md). Keep changes small, preserve existing working behavior, add tests for policy/geometry/runtime logic, and do not weaken the local-first/security boundaries for convenience.

## Licence

Still2Solid application code is licensed under **Apache-2.0**. Model weights and third-party runtime components are separately licensed and do not inherit the application licence. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and [Model Licence Policy](docs/MODEL_LICENSE_POLICY.md).
