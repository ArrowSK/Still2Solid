# Still2Solid

Local-first image-to-3D desktop application.

Still2Solid is being built as a hardware-aware, model-agnostic workflow: drop an image, choose quality, generate a 3D asset, preview it locally, and export it without requiring cloud inference.

## Status

Milestone M2 adds a hardware-aware Model Manager on top of the M1 desktop shell. The application now detects local accelerator capabilities, assesses a curated production-model catalogue, explains compatibility and licence constraints, and can remember a preferred production candidate.

M2 deliberately keeps deterministic Mock3D as the only executable adapter. Production model downloads, checksum-verified installation and isolated inference workers are M3 work; the UI does not pretend otherwise.

## Licence

Apache-2.0 for Still2Solid application code. Model weights are separately licensed and are never assumed to inherit the application licence.
