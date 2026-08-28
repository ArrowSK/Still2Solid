# Architecture

Still2Solid is a local-first desktop application. The stable user workflow is intentionally separated from model-specific inference code.

## Layers

1. **Desktop UI** — Svelte/TypeScript rendered in the native Tauri webview. Owns image selection, simple/advanced controls, progress presentation, Model Manager and Three.js preview.
2. **Tauri core** — Rust process that owns trusted native capabilities such as hardware probing, accelerator discovery, future worker lifecycle management, downloads, checksums and local filesystem access.
3. **Model catalogue and policy** — declarative model metadata plus deterministic hardware/licence assessment. It can recommend a production candidate without importing or running that model.
4. **Model adapters** — replaceable inference implementations conforming to a common manifest and progress contract. M1/M2 ship only executable `Mock3D`.
5. **Model workers** — production adapters will run in isolated local worker processes rather than an open localhost server.
6. **Asset pipeline** — future normalization to GLB, mesh repair, print preparation and export.

## M2 boundary

M2 builds on the M1 shell and adds:

- native memory, Apple unified-GPU and NVIDIA/CUDA discovery;
- a curated production-model catalogue;
- explicit `Recommended`, `Compatible`, `Compatible · slow path`, `Memory constrained` and `Unsupported` assessments;
- licence-aware automatic selection that does not silently choose gated or conditional models;
- a Model Manager that explains every decision and remembers an explicit user preference for M3.

M2 does **not** download production weights, create Python/model runtimes, or execute AI inference. Mock3D remains the only runnable adapter until M3 adds pinned downloads, checksum verification and isolated model workers.

This boundary prevents a model-download feature from being confused with a working inference implementation and keeps low-memory machines able to run the application even when no production candidate is safe.

## Security invariants

- Source images stay local.
- No telemetry or analytics.
- No cloud inference.
- No localhost HTTP inference service.
- No automatic execution of Hugging Face remote code.
- External model weights must be separately licensed, pinned and checksum-verified before production use.
- Conditional or gated model licences are never silently accepted on the user's behalf.
