# Still2Solid

Local-first image-to-3D desktop application.

Still2Solid is being built as a hardware-aware, model-agnostic workflow: drop an image, choose quality, generate a 3D asset, preview it locally, and export it without requiring cloud inference.

## Status

Milestone M5 adds a canonical production asset/export layer on top of the M3 TripoSR runtime and M4 learned ETA system.

A successful production generation is treated as a validated GLB 2.0 master. Still2Solid can now inspect that master and export it non-destructively as:

- the exact GLB master;
- an OBJ compatibility package containing OBJ, MTL, available PNG textures and an asset manifest;
- binary STL geometry for downstream mesh/printing tools.

OBJ/MTL is explicitly treated as a compatibility export rather than a fidelity-preserving format because it cannot represent the full GLB/PBR material model. STL is explicitly geometry-only and does not provide reliable unit metadata. Print repair, sizing and 3MF belong to M6.

Production inference still runs in a one-shot isolated local Python process. Still2Solid does not expose a localhost inference server, enable telemetry, or allow the worker to fetch Hugging Face code or weights during generation. Learned timing data remains local-only.

## Licence

Apache-2.0 for Still2Solid application code. Model weights and third-party runtime components are separately licensed and never assumed to inherit the application licence. See `MODEL_LICENSE_POLICY.md` and `THIRD_PARTY_NOTICES.md`.
