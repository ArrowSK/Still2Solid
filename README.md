# Still2Solid

Local-first image-to-3D desktop application.

Still2Solid is being built as a hardware-aware, model-agnostic workflow: drop an image, choose quality, generate a 3D asset, preview it locally, and export it without requiring cloud inference.

## Status

Milestone M4 adds local learned timing profiles on top of the M3 TripoSR production runtime. Successful generations are timed by stage and grouped by exact hardware profile, model/version, quality, backend choice and foreground-isolation setting. Those local profiles replace generic fixed ETA guesses as comparable runs accumulate.

Failed and cancelled jobs are never used for ETA learning. Extreme successful timing outliers are retained only as excluded diagnostics and do not affect the learned median. Timing history is stored in the application's local browser storage only; Still2Solid sends no timing telemetry.

Production inference still runs in a one-shot isolated local Python process. Still2Solid does not expose a localhost inference server or allow the worker to fetch Hugging Face code or weights during generation.

M4 remains a development milestone. The in-app runtime installer currently uses an existing Python 3.11 or 3.12 interpreter only to create an isolated environment. Bundled interpreter/runtime packaging is deferred to the release-packaging milestone.

## Licence

Apache-2.0 for Still2Solid application code. Model weights and third-party runtime components are separately licensed and never assumed to inherit the application licence. See `MODEL_LICENSE_POLICY.md` and `THIRD_PARTY_NOTICES.md`.
