# Architecture

Still2Solid is a local-first desktop application. The stable user workflow is intentionally separated from model-specific inference code, timing intelligence, canonical asset handling and downstream print preparation.

## Layers

1. **Desktop UI** — Svelte/TypeScript in the Tauri webview. Owns image selection, simple/advanced controls, progress presentation, Model Manager, Three.js preview, export UI and print-preparation controls.
2. **Tauri core** — Rust process that owns trusted native capabilities: hardware probing, runtime installation, checksum verification, worker lifecycle, cancellation and app-local filesystem access.
3. **Model catalogue and policy** — declarative model metadata plus deterministic hardware/licence assessment. Models can be assessed without installing or executing them.
4. **Model adapters** — replaceable implementations conforming to the common generation/progress contract. Current executable adapters are Mock3D and production TripoSR.
5. **Model workers** — production inference runs in isolated one-shot local processes, never an open localhost service.
6. **Timing intelligence** — local-only per-hardware/model/quality/backend timing profiles that improve progress and ETA without altering inference parameters.
7. **Canonical asset layer** — successful production output is treated as a validated GLB 2.0 master. Preview and downstream conversions derive from this master without mutating it.
8. **Derived export layer** — creates compatibility/output formats such as OBJ/MTL/PNG ZIP and raw STL in memory while preserving the GLB master.
9. **Print-preparation layer** — creates a separate geometry-only prepared copy with explicit millimetre sizing, printer-oriented coordinates, topology analysis, conservative repair, optional base flattening and 3MF/prepared-STL output.

## Production runtime boundary

The audited production path remains TripoSR. The installer pins source and model revisions, verifies downloaded source/model assets before activation, and runs inference fully locally from the installed runtime. Timing profiles remain a UI-side local-only learning layer and do not alter inference settings.

## Canonical asset boundary

Still2Solid maintains one core invariant:

> The model worker's validated GLB 2.0 output is the fidelity-preserving master asset for the generation.

The GLB master is never rewritten merely because the user requests another export or print-preparation operation.

### GLB export

GLB export downloads the exact validated worker output. It remains the preferred format for preserving embedded materials and textures.

### OBJ compatibility package

OBJ/MTL cannot represent the full glTF/PBR material model. Conversion therefore remains explicitly lossy and compatibility-oriented. Geometry, normals, UVs, approximated MTL data and browser-readable PNG textures are packed locally into a ZIP with an asset manifest.

### Raw STL export

Raw STL is a direct geometry derivative of the canonical GLB. It intentionally carries no promise of physical scale, colour, texture or print readiness.

## M6 print-preparation boundary

M6 introduces a second invariant:

> Print preparation operates only on an in-memory copy of canonical geometry and never changes the canonical GLB.

The generated single-image mesh has no trustworthy physical scale. Still2Solid therefore requires the user to choose the intended longest dimension in millimetres. The prepared copy is scaled isotropically, converted from Three.js/glTF Y-up coordinates to printer-friendly Z-up coordinates and translated so its lowest point rests at Z=0.

Optional 90° rotations are applied after the Y-up → Z-up conversion. This makes orientation deterministic and avoids pretending that the source photograph establishes a physically correct print orientation.

### Topology analysis

The M6 analyser works on an indexed geometry-only representation and reports:

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

The optional flat-base setting clamps only the lowest user-selected millimetre band of the prepared copy to a plane. This is intentionally simpler and more predictable than a hidden boolean cut. Degenerate faces created by that operation are removed and a resulting planar base boundary is capped only when the conservative loop rules allow it.

### 3MF export

M6 writes a minimal 3MF Core package locally using the already-present `fflate` ZIP implementation. The package contains:

- `[Content_Types].xml`;
- `_rels/.rels`;
- `3D/3dmodel.model`.

The 3MF model declares `unit="millimeter"` explicitly and contains the prepared vertices and triangles. No new third-party 3MF library or cloud service is required.

### Prepared STL export

Prepared STL contains the same scaled/oriented/repaired triangle geometry as the 3MF export. Coordinates are numerically expressed in millimetres, but STL has no unit metadata, so the UI explicitly tells the user to import it as millimetres.

## Preserved behaviour

If TripoSR is not installed, not verified or not selected, the Mock3D workflow remains available. M6 does not replace the production runtime, learned ETA system, Model Manager, quality presets, canonical GLB preview or M5 raw export options.

## Security and privacy invariants

- Source images stay local.
- No telemetry or analytics.
- No cloud inference.
- No localhost HTTP inference service.
- No `trust_remote_code=True`.
- Production source and weights remain pinned and checksum verified before execution.
- Export conversion and print preparation happen in memory inside the local desktop application.
- Generated geometry is not uploaded.
- The canonical GLB is not modified by OBJ/STL conversion or print preparation.
