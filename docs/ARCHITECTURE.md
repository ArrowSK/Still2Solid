# Architecture

Still2Solid is a local-first desktop application. The stable user workflow is intentionally separated from model-specific inference code.

## Layers

1. **Desktop UI** — Svelte/TypeScript in the Tauri webview. Owns image selection, simple/advanced controls, progress presentation, Model Manager and Three.js preview.
2. **Tauri core** — Rust process that owns trusted native capabilities: hardware probing, runtime installation, checksum verification, worker lifecycle, cancellation and app-local filesystem access.
3. **Model catalogue and policy** — declarative model metadata plus deterministic hardware/licence assessment. Models can be assessed without installing or executing them.
4. **Model adapters** — replaceable implementations conforming to the common generation/progress contract. M3 ships Mock3D plus the production TripoSR adapter.
5. **Model workers** — production inference runs in isolated one-shot local processes, never an open localhost service.
6. **Asset pipeline** — M3 can preview/export the direct TripoSR GLB; broader normalization, repair and export belongs to M5.

## M3 production boundary

M3 implements one audited production path only: TripoSR.

The installer:

- creates an isolated Python environment;
- downloads the exact TripoSR source revision `107cefdc244c39106fa830359024f6a2f1c78871` file-by-file;
- verifies each source file against its pinned Git blob SHA-1;
- downloads model revision `5b521936b01fbe1890f6f9baed0254ab6351c04a`;
- verifies `model.ckpt` with SHA-256 `429e2c6b22a0923967459de24d67f05962b235f79cde6b032aa7ed2ffcd970ee`;
- downloads U2Net for optional foreground isolation and verifies the checksum published by rembg;
- writes an installation manifest only after all verification succeeds;
- atomically promotes the staging installation to the active model directory.

The worker:

- receives only a local normalized source image and explicit generation settings;
- blocks Hugging Face runtime downloads and runs with offline environment flags;
- loads only the pinned local checkpoint/configuration;
- uses a CPU scikit-image marching-cubes shim instead of requiring the upstream `torchmcubes` CUDA extension;
- selects CUDA, Metal/MPS or CPU at runtime according to the requested backend and actual PyTorch availability;
- uses conservative Fast/Standard/Best extraction settings;
- attempts UV texture baking and falls back to a valid vertex-colour GLB if texture baking is unavailable;
- exits after every generation, thereby unloading the model automatically.

The Tauri core owns the child process and can terminate it on cancellation. Progress is emitted from the worker to the UI through Tauri events, not through a network port.

## Preserved behaviour

If TripoSR is not installed, not verified or not selected, the M1/M2 Mock3D workflow remains available. Existing image selection, quality controls, cancellation and preview behaviour are not replaced merely because the production layer is absent.

## Security invariants

- Source images stay local.
- No telemetry or analytics.
- No cloud inference.
- No localhost HTTP inference service.
- No `trust_remote_code=True`.
- Production source and weights are pinned before execution.
- Downloaded TripoSR source files and model weights are checksum verified before activation.
- Conditional/gated models are never silently accepted or installed.
- The only M3 installable model ID is `triposr`; arbitrary model URLs or executable paths are not accepted by the Tauri commands.
