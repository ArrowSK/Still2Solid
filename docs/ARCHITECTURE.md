# Architecture

Still2Solid is a local-first desktop application. The stable user workflow is intentionally separated from model-specific inference code.

## Layers

1. **Desktop UI** — Svelte/TypeScript rendered in the native Tauri webview. Owns image selection, simple/advanced controls, progress presentation and Three.js preview.
2. **Tauri core** — Rust process that owns trusted native capabilities such as hardware probing, future worker lifecycle management, downloads, checksums and local filesystem access.
3. **Model adapters** — replaceable inference implementations conforming to a common manifest and progress contract. M1 ships only `Mock3D`.
4. **Model workers** — future production adapters run in isolated local worker processes rather than an open localhost server.
5. **Asset pipeline** — future normalization to GLB, mesh repair, print preparation and export.

## M1 boundary

M1 deliberately does not run AI inference. The deterministic Mock3D adapter validates:

- image drop/select workflow;
- quality presets;
- advanced-mode shell;
- model explanation surface;
- progress stage contract, ETA and cancellation;
- hardware probe through Tauri IPC;
- interactive Three.js model preview;
- local mock GLB export.

This prevents production-model constraints from becoming UI architecture.

## Security invariants

- Source images stay local.
- No telemetry or analytics.
- No cloud inference.
- No localhost HTTP inference service.
- No automatic execution of Hugging Face remote code.
- External model weights must be separately licensed and pinned before production use.
