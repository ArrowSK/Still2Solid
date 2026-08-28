# Documentation index

Still2Solid's documentation is arranged so you can stop reading as soon as you have what you need.

## I just want to run it

Start with [Getting Started](GETTING_STARTED.md), then keep the [User Guide](USER_GUIDE.md) nearby for the workflow itself.

## I want to understand model choice

Read [Models & Hardware](MODELS_AND_HARDWARE.md). It explains the meaning of Recommended, Compatible, Memory constrained and Licence restricted, plus the current TripoSR/SF3D/TRELLIS position.

## Something is not working

Use [Troubleshooting](TROUBLESHOOTING.md). It starts with non-destructive checks for runtime installation, Apple Silicon memory/MPS, CUDA detection, background removal, generation, exports and print topology.

## I want to work on the code

Read these in order:

1. [Development](DEVELOPMENT.md)
2. [Architecture](ARCHITECTURE.md)
3. [Security & Privacy](SECURITY_PRIVACY.md)
4. [Contributing](../CONTRIBUTING.md)
5. [Model Licence Policy](MODEL_LICENSE_POLICY.md) if models/runtimes are involved

## I want to understand background removal

Read [Background Guidance](BACKGROUND_GUIDANCE.md). The short version: the suggestion is a tiny local heuristic; actual foreground isolation is the already-existing local production preprocessing option; the user always controls it.

## I want to understand exports or printing

The [User Guide](USER_GUIDE.md) explains the normal experience. For implementation details, read [Architecture](ARCHITECTURE.md), then the historical [M5](M5.md) and [M6](M6.md) milestone notes.

## I want to know what is finished

Read [Roadmap](ROADMAP.md). It separates completed M1–M6 functionality, the open 8 GB Apple Silicon validation gate, planned M7 release packaging and future M8 model expansion.

## I want the implementation history

The milestone documents are retained as engineering records:

- [M1](M1.md) — desktop shell and Mock3D
- [M2](M2.md) — Model Manager and hardware policy
- [M3](M3.md) — TripoSR production runtime
- [M4](M4.md) — learned timing/ETA
- [M5](M5.md) — canonical assets and exports
- [M6](M6.md) — print preparation and 3MF

They are useful for understanding why a boundary exists, but the current user-facing truth should be taken from the README, User Guide, Architecture and Roadmap when wording has evolved since an earlier milestone.

## Branding and licences

- [Branding](BRANDING.md)
- [Model Licence Policy](MODEL_LICENSE_POLICY.md)
- [Third-party notices](../THIRD_PARTY_NOTICES.md)
- [Apache-2.0 licence](../LICENSE)

## Security reporting

See [SECURITY.md](../SECURITY.md).
