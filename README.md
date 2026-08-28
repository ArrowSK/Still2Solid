# Still2Solid

Local-first image-to-3D desktop application.

Still2Solid is being built as a hardware-aware, model-agnostic workflow: drop an image, choose quality, generate a 3D asset, preview it locally, and export it without requiring cloud inference.

## Status

Milestone M3 adds the first real production adapter: TripoSR. The Model Manager can install a pinned, checksum-verified TripoSR runtime, and Generate switches from Mock3D to TripoSR only when that runtime is installed, verified and selected.

Production inference runs in a one-shot isolated local Python process. Still2Solid does not expose a localhost inference server, enable telemetry, or allow the worker to fetch Hugging Face code or weights during generation.

M3 is still a development milestone. Its in-app runtime installer currently uses an existing Python 3.11 or 3.12 interpreter only to create an isolated environment. Bundled interpreter/runtime packaging is deferred to the release-packaging milestone.

## Licence

Apache-2.0 for Still2Solid application code. Model weights and third-party runtime components are separately licensed and never assumed to inherit the application licence. See `MODEL_LICENSE_POLICY.md` and `THIRD_PARTY_NOTICES.md`.
