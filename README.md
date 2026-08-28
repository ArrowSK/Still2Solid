# Still2Solid

Local-first image-to-3D desktop application.

Still2Solid is being built as a hardware-aware, model-agnostic workflow: drop an image, choose quality, generate a 3D asset, preview it locally, prepare it for downstream use, and export it without requiring cloud inference.

## Status

Milestone M6 adds a non-destructive print-preparation layer on top of the M5 canonical GLB/export pipeline.

A successful production generation remains a validated GLB 2.0 master. Print preparation now creates a separate in-memory geometry copy with explicit user-controlled sizing in millimetres, printer-oriented Z-up coordinates and build-plate placement at Z=0.

M6 can now:

- set the intended longest dimension explicitly in millimetres;
- apply 90° orientation adjustments around X/Y/Z;
- optionally flatten a shallow lowest band to form a more stable base;
- analyse watertightness, edge and vertex manifoldness, degenerate faces, disconnected shells and winding conflicts;
- conservatively remove degenerate faces, repair consistent winding, orient closed shells outward and cap simple planar holes when safe;
- distinguish **Printable** from **Automatic repair incomplete** instead of silently declaring every generated mesh print-ready;
- export a prepared 3MF package with explicit millimetre units;
- export a prepared binary STL whose coordinates are scaled in millimetres, while still warning that STL itself stores no unit metadata.

The source photo does not contain reliable physical scale, so Still2Solid never guesses real-world dimensions from the photograph. The user chooses the target size explicitly.

The M5 raw export path remains unchanged: the exact GLB master, OBJ compatibility ZIP and raw STL are still available, and print preparation never modifies the canonical GLB.

Production inference still runs in a one-shot isolated local Python process. Still2Solid does not expose a localhost inference server, enable telemetry, or allow the worker to fetch Hugging Face code or weights during generation. Learned timing data and print-preparation analysis remain local-only.

## Licence

Apache-2.0 for Still2Solid application code. Model weights and third-party runtime components are separately licensed and never assumed to inherit the application licence. See `MODEL_LICENSE_POLICY.md` and `THIRD_PARTY_NOTICES.md`.
