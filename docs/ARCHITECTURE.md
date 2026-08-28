# Architecture

Still2Solid is a local-first desktop application. The stable user workflow is intentionally separated from model-specific inference code and from downstream export/printing concerns.

## Layers

1. **Desktop UI** — Svelte/TypeScript in the Tauri webview. Owns image selection, simple/advanced controls, progress presentation, Model Manager, Three.js preview and M5 export UI.
2. **Tauri core** — Rust process that owns trusted native capabilities: hardware probing, runtime installation, checksum verification, worker lifecycle, cancellation and app-local filesystem access.
3. **Model catalogue and policy** — declarative model metadata plus deterministic hardware/licence assessment. Models can be assessed without installing or executing them.
4. **Model adapters** — replaceable implementations conforming to the common generation/progress contract. Current executable adapters are Mock3D and production TripoSR.
5. **Model workers** — production inference runs in isolated one-shot local processes, never an open localhost service.
6. **Canonical asset layer** — successful production output is treated as a validated GLB 2.0 master. Preview and exports derive from this master without mutating it.
7. **Derived export layer** — M5 creates compatibility/output formats from the canonical GLB in memory. Print repair, sizing and 3MF remain separate M6 work.

## Production runtime boundary

The audited production path remains TripoSR. The installer pins source and model revisions, verifies downloaded source/model assets before activation, and runs inference fully locally from the installed runtime. M4 timing profiles remain a UI-side, local-only learning layer and do not alter inference parameters.

## M5 canonical asset boundary

M5 introduces one explicit invariant:

> The model worker's validated GLB 2.0 output is the fidelity-preserving master asset for the generation.

Before presenting export options, the UI validates the GLB header and parses the asset locally. The inspector records only in-memory technical facts such as byte size, mesh count, vertex/triangle count, material count, texture count and geometric bounds.

The master is not rewritten simply because another format is requested.

### GLB export

The GLB export downloads the exact validated bytes produced by the worker. It is therefore the preferred format for preserving embedded textures/materials and the closest representation of the generated result.

### OBJ compatibility package

OBJ/MTL cannot represent the full glTF/PBR material model. M5 therefore labels it as a compatibility export rather than an equivalent master.

Conversion is performed locally in the UI using Three.js:

- geometry, normals and UVs are exported to OBJ;
- stable material names are generated;
- MTL approximates base colour and opacity;
- browser-readable base-colour and normal maps are converted to PNG and referenced from MTL;
- OBJ, MTL, textures and `asset.json` are packed into a single ZIP using fflate;
- the canonical GLB is unchanged.

If a texture cannot be read back by the browser, the package remains structurally valid but cannot claim that texture was preserved. The manifest states that OBJ/MTL is a lossy compatibility conversion.

### STL export

M5 can derive binary STL geometry from the canonical GLB. The UI explicitly warns that STL carries no texture/colour and no reliable unit metadata.

M5 does not claim STL is print-ready. Mesh repair, manifold/watertight checks, explicit sizing/orientation and 3MF are M6.

## Preserved behaviour

If TripoSR is not installed, not verified or not selected, the Mock3D workflow remains available. M5 does not replace the production runtime, learned ETA system, Model Manager, quality presets or preview controls.

## Security and privacy invariants

- Source images stay local.
- No telemetry or analytics.
- No cloud inference.
- No localhost HTTP inference service.
- No `trust_remote_code=True`.
- Production source and weights remain pinned and checksum verified before execution.
- Export conversion happens in memory inside the local desktop application.
- Export does not upload generated geometry or textures.
- The canonical master is not modified by OBJ/STL conversion.
