<p align="center">
  <img src="assets/branding/still2solid-logo.svg" alt="Still2Solid — local image to 3D" width="520">
</p>

<p align="center">
  <strong>Turn one image into a local, textured 3D asset — then preview, export or prepare it for printing.</strong>
</p>

<p align="center">
  <a href="https://github.com/ArrowSK/Still2Solid/releases/download/v0.8.2/Still2Solid_0.8.2_aarch64.dmg"><img alt="Download Still2Solid for macOS Apple Silicon" src="https://img.shields.io/badge/Download-macOS%20Apple%20Silicon-111827?style=for-the-badge&logo=apple&logoColor=white"></a>
  <a href="https://github.com/ArrowSK/Still2Solid/releases/tag/v0.8.2"><img alt="View GitHub Release" src="https://img.shields.io/badge/GitHub-v0.8.2-2f81f7?style=for-the-badge&logo=github&logoColor=white"></a>
</p>

<p align="center">
  <sub><strong>macOS:</strong> Apple Silicon (M1/M2/M3/M4). Open the DMG and drag Still2Solid to Applications. The current build is unsigned, so macOS may require <em>Open Anyway</em> on first launch.</sub>
</p>

<p align="center">
  <img alt="Version 0.8.2" src="https://img.shields.io/badge/version-0.8.2-4d8dff">
  <img alt="Apache 2.0" src="https://img.shields.io/badge/license-Apache--2.0-6c7a89">
  <img alt="Local first" src="https://img.shields.io/badge/inference-local--first-19b5a5">
  <img alt="No telemetry" src="https://img.shields.io/badge/telemetry-none-19b5a5">
  <a href="https://github.com/ArrowSK/Still2Solid/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/ArrowSK/Still2Solid/actions/workflows/ci.yml/badge.svg"></a>
</p>

Still2Solid is a desktop image-to-3D application built around a deliberately simple workflow: choose an image, let the app assess the computer, generate locally, inspect the result and export it. The application is designed to stay understandable even though the model runtimes underneath it are not simple.

> **Current state:** M1–M8 are implemented. **Still2Solid v0.8.2** is published on GitHub Releases for Apple Silicon macOS with a SHA-256 checksum. The current macOS build is unsigned; Apple Developer signing and notarization remain a distribution-hardening task rather than a separate product edition. The physical M1/8 GB benchmark remains an explicit validation task for performance guidance.

## Install on macOS

For an Apple Silicon Mac, use the **Download** button above or open the [v0.8.2 release](https://github.com/ArrowSK/Still2Solid/releases/tag/v0.8.2).

1. Download `Still2Solid_0.8.2_aarch64.dmg`.
2. Open the DMG.
3. Drag **Still2Solid** to **Applications**.
4. Launch it from Applications or Launchpad.
5. If macOS blocks the unsigned build, use **System Settings → Privacy & Security → Open Anyway** once.

Python is bundled in the packaged app. Production model weights are installed from inside Still2Solid when you choose to install a model.

For a normal app-only uninstall, quit Still2Solid and move it from **Applications** to **Trash**. Downloaded models are intentionally kept so reinstalling the app does not force multi-gigabyte downloads. For a complete uninstall, open **Settings → Storage → Prepare for uninstall** first. Still2Solid removes its downloaded models, app data, cache, abandoned temporary work and local preferences; then quit the app and move it to Trash.

## What it does today

- Local image selection and drag-and-drop workflow.
- Hardware-aware Model Manager and conservative compatibility policy.
- Settings → Storage with model removal, temporary-file cleanup and a complete-uninstall preparation flow.
- Automatic startup cleanup for abandoned generation jobs and interrupted model-install staging.
- Model installation avoids shared pip download caches; model support caches required for offline inference stay model-owned.
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

If you want to **use Still2Solid on an Apple Silicon Mac**, download the DMG at the top of this page. If you are building from source, read **[Getting Started](docs/GETTING_STARTED.md)**. If you mainly want to understand the product workflow, read the **[User Guide](docs/USER_GUIDE.md)**. Developers should continue with **[Development](docs/DEVELOPMENT.md)** and **[Architecture](docs/ARCHITECTURE.md)**.

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

Temporary generation workspaces and interrupted `.installing` staging directories are Still2Solid-owned and are automatically removed on the next launch after a crash, force-quit or power loss. **Settings → Storage → Clear temporary files** also removes abandoned work plus the normal app cache. Required SF3D support caches are model-owned so model uninstall removes them without touching unrelated user caches.

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
| macOS v0.8.2 | Published | Apple Silicon DMG and SHA-256 checksum are available from GitHub Releases. |
| Target-device validation | Open | Measure the physical M1/8 GB experience before changing its conservative recommendation. |
| Apple signing/notarization | Open | Configure Developer ID signing and notarization for a frictionless first-launch experience. |

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md). Keep changes small, preserve existing working behavior, add tests for policy/geometry/runtime logic, and do not weaken local-first/security boundaries for convenience.

## Licence

Still2Solid application code is licensed under **Apache-2.0**. Model weights and third-party runtime components are separately licensed and do not inherit the application licence. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and [Model Licence Policy](docs/MODEL_LICENSE_POLICY.md).