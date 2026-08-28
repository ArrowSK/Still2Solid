# Architecture

Still2Solid is a local-first desktop application. The visible workflow is intentionally simple, but the implementation keeps model-specific inference, trusted native operations, timing intelligence, canonical asset handling and print preparation in separate layers.

The main design rule is straightforward:

> A failure in one layer should not force Still2Solid to replace a different layer that is already working.

## System map

```text
Svelte/Tauri UI
  ├─ local image + Background check
  ├─ hardware/model policy presentation
  ├─ progress + learned ETA
  ├─ Three.js preview
  ├─ export UI
  └─ print-prep UI
          │
          ▼
Trusted Rust/Tauri core
  ├─ hardware probe
  ├─ allowlisted installer
  ├─ hash/checksum verification
  ├─ app-local runtime paths
  └─ worker lifecycle / cancellation
          │
          ▼
One-shot model worker
  └─ pinned local TripoSR runtime + weights
          │
          ▼
Validated GLB 2.0 master
  ├─ exact GLB
  ├─ OBJ compatibility package
  ├─ raw STL
  └─ prepared print copy → 3MF / prepared STL
```

## Layers

1. **Desktop UI** — Svelte/TypeScript in the Tauri webview. Owns image selection, simple/advanced controls, Background check guidance, progress presentation, Model Manager, Three.js preview, export UI and print-preparation controls.
2. **Local background guidance** — a lightweight pixel heuristic that inspects a downscaled local copy for transparency and edge/centre statistics. It suggests foreground isolation but never blocks or overrides the user permanently.
3. **Tauri core** — Rust process that owns trusted native capabilities: hardware probing, runtime installation, checksum verification, worker lifecycle, cancellation and app-local filesystem access.
4. **Model catalogue and policy** — declarative model metadata plus deterministic hardware/licence assessment. Models can be assessed without installing or executing them.
5. **Model adapters** — replaceable implementations conforming to the common generation/progress contract. Current executable adapters are Mock3D and production TripoSR.
6. **Model workers** — production inference runs in isolated one-shot local processes, never an open localhost service.
7. **Timing intelligence** — local-only per-hardware/model/quality/backend/background-removal timing profiles that improve progress and ETA without altering inference parameters.
8. **Canonical asset layer** — successful production output is treated as a validated GLB 2.0 master. Preview and downstream conversions derive from this master without mutating it.
9. **Derived export layer** — creates compatibility/output formats such as OBJ/MTL/PNG ZIP and raw STL in memory while preserving the GLB master.
10. **Print-preparation layer** — creates a separate geometry-only prepared copy with explicit millimetre sizing, printer-oriented coordinates, topology analysis, conservative repair, optional base flattening and 3MF/prepared-STL output.

## Background guidance boundary

Background handling has two distinct parts and they should not be confused.

### Background check

`backgroundAnalysis.ts` downsamples the selected local image to a tiny canvas and examines:

- transparency at the image edge;
- colour variation around the border;
- colour distance between the border and centre.

The result is one of:

- likely background;
- already transparent/isolated;
- uncertain.

This is deliberately a **heuristic**, not a second segmentation model. It is fast, local and explainable enough to provide guidance without adding another heavyweight runtime or cloud dependency.

The UI exposes the resulting recommendation through `BackgroundAdvisor.svelte`. The user can always change the setting.

### Foreground isolation

The actual foreground-removal operation remains part of the production generation path and uses the installed local U2Net/rembg asset when enabled.

The background heuristic does not modify the source image itself. It only chooses/recommends the value of the existing generation option.

## Production runtime boundary

The audited production path remains TripoSR. The installer pins source and model revisions, verifies downloaded source/model assets before activation, and runs inference fully locally from the installed runtime.

The current development installer needs a discoverable Python 3.11/3.12 only to create the private environment. After creation the production worker uses that private runtime. M7 is intended to bundle the end-user runtime so this development prerequisite disappears from normal installation.

Production inference is a one-shot child process. Still2Solid does not expose an HTTP inference server, aggressive reconnect loop or persistent model daemon.

## Model policy boundary

Model availability is not the same as model recommendation.

The catalogue separately tracks:

- hardware compatibility;
- memory pressure;
- backend support;
- licence status;
- gating/availability;
- Still2Solid validation confidence.

Conditional/gated models are not silently auto-selected. Regionally incompatible models are excluded from the official catalogue. Unknown hardware does not receive a fake recommendation.

See [Models & Hardware](MODELS_AND_HARDWARE.md) and [Model Licence Policy](MODEL_LICENSE_POLICY.md).

## Timing intelligence boundary

M4 timing profiles are UI-side local data. They are keyed by the conditions that materially affect runtime, including hardware, model/version, quality, backend and foreground-isolation choice.

Only successful jobs train the profile. Failed and cancelled runs are excluded. The profile is bounded and should never contain source image content or filenames.

Timing intelligence may improve progress/ETA presentation; it must not silently modify inference settings just to make an ETA come true.

## Canonical asset boundary

Still2Solid maintains a core invariant:

> The model worker's validated GLB 2.0 output is the fidelity-preserving master asset for the generation.

The GLB master is never rewritten merely because the user requests another export or print-preparation operation.

### GLB export

GLB export downloads the exact validated worker output. It remains the preferred format for preserving embedded materials and textures.

### OBJ compatibility package

OBJ/MTL cannot represent the full glTF/PBR material model. Conversion therefore remains explicitly lossy and compatibility-oriented. Geometry, normals, UVs, approximated MTL data and browser-readable PNG textures are packed locally into a ZIP with an asset manifest.

### Raw STL export

Raw STL is a direct geometry derivative of the canonical GLB. It intentionally carries no promise of physical scale, colour, texture or print readiness.

## Print-preparation boundary

M6 introduces a second invariant:

> Print preparation operates only on an in-memory copy of canonical geometry and never changes the canonical GLB.

The generated single-image mesh has no trustworthy physical scale. Still2Solid therefore requires the user to choose the intended longest dimension in millimetres. The prepared copy is scaled isotropically, converted from Three.js/glTF Y-up coordinates to printer-friendly Z-up coordinates and translated so its lowest point rests at Z=0.

Optional 90° rotations are applied after the Y-up → Z-up conversion. This makes orientation deterministic and avoids pretending that the source photograph establishes a physically correct print orientation.

### Topology analysis

The analyser works on an indexed geometry-only representation and reports:

- triangle and vertex counts;
- degenerate triangles;
- open/boundary edges;
- non-manifold edges;
- disconnected vertex fans (non-manifold vertices);
- winding/orientation conflicts;
- disconnected shells/components;
- watertight and manifold status;
- enclosed volume when the prepared mesh is closed and manifold.

The `Printable` label is used only when the prepared geometry contains triangles and has no degenerate faces, open boundaries, non-manifold edges/vertices or winding conflicts. Multiple closed shells are allowed but reported as a warning because they may be intentional or accidental.

### Conservative automatic repair

Automatic repair is deliberately limited. M6 may:

1. remove zero-area/degenerate faces;
2. propagate consistent triangle winding across connected faces;
3. orient closed shells outward using signed volume;
4. optionally cap boundary loops only when they are simple, sufficiently planar and conservatively bounded in size;
5. treat a planar loop created along the explicit flat-base plane as eligible for capping.

M6 does **not** perform arbitrary boolean remeshing, aggressive hole filling, voxelisation, sculpting or topology invention. If topology remains open or non-manifold, status becomes `Automatic repair incomplete` and the user may still export for manual repair in a dedicated mesh tool or slicer.

### Flat base

The optional flat-base setting clamps only the lowest user-selected millimetre band of the prepared copy to a plane. Degenerate faces created by that operation are removed and a resulting planar base boundary is capped only when the conservative loop rules allow it.

### 3MF export

M6 writes a minimal 3MF Core package locally using the already-present `fflate` ZIP implementation. The package contains:

- `[Content_Types].xml`;
- `_rels/.rels`;
- `3D/3dmodel.model`.

The 3MF model declares `unit="millimeter"` explicitly and contains the prepared vertices and triangles. No cloud service or separate 3MF library is required.

### Prepared STL export

Prepared STL contains the same scaled/oriented/repaired triangle geometry as the 3MF export. Coordinates are numerically expressed in millimetres, but STL has no unit metadata, so the UI explicitly tells the user to import it as millimetres.

## Application branding boundary

Brand artwork is presentation-only. The in-app WebP icon and repository wordmark live separately from the native Tauri PNG icon source. Branding must not become a dependency of generation/runtime logic.

See [Branding](BRANDING.md).

## Release boundary

The current repository is functionally through M6 but is still a development build from an end-user distribution perspective.

M7 should own:

- bundled runtime packaging;
- native platform icon derivatives;
- signing/notarization;
- release artifacts/workflow;
- first-run install/recovery behavior.

Those release concerns should not be solved by weakening model verification or moving inference to the cloud.

## Preserved behaviour

If TripoSR is not installed, not verified or not selected, the Mock3D workflow remains available. Background guidance, timing, export and print-preparation changes must preserve that fallback rather than making the app unusable when production inference is unavailable.

## Security and privacy invariants

- Source images stay local.
- Background assessment stays local.
- No telemetry or analytics.
- No cloud inference.
- No localhost HTTP inference service.
- No `trust_remote_code=True` production model loading.
- Production source and weights remain pinned and checksum verified before execution.
- Export conversion and print preparation happen locally inside the desktop application.
- Generated geometry is not uploaded by the core flow.
- The canonical GLB is not modified by OBJ/STL conversion or print preparation.

See [Security & Privacy](SECURITY_PRIVACY.md) for the complete policy explanation.
