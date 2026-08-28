# Roadmap

Still2Solid uses milestones as capability boundaries. A milestone is complete when its code, tests and user-facing behavior exist; that does not convert unmeasured hardware claims or unsigned release artifacts into facts.

## Where the project is now

M1 through M8 are implemented in the repository:

- image-first desktop workflow and Mock3D fallback;
- hardware-aware model policy;
- verified local TripoSR production path;
- learned local progress and ETA;
- canonical GLB plus derived exports;
- non-destructive print preparation and 3MF;
- local background guidance;
- canonical branding in the repository, web UI and native package icons;
- bundled, checksum-verified Python runtime preparation and active Tauri packaging;
- cross-platform release workflow with macOS signing/notarization hooks;
- an audited second production adapter for gated Stable Fast 3D.

The remaining items are validation/operations rather than missing M1–M8 feature code: physical low-memory Apple-Silicon benchmarking and actually producing signed/notarized public artifacts with configured release credentials.

## Completed milestones

### M1 — Product shell and Mock3D

Delivered the simple image-first desktop workflow, quality controls, deterministic Mock3D adapter, progress/cancel flow and interactive preview.

### M2 — Model Manager and hardware policy

Added native hardware probing, curated model catalogue, deterministic compatibility states and conservative automatic recommendation rules.

### M3 — Production TripoSR adapter

Added the first real production model path with immutable source/model revisions, integrity verification, private runtime environment, one-shot worker, foreground isolation, backend selection, canonical production GLB and safe Mock3D fallback.

### M4 — Learned progress and ETA

Added per-machine/per-setting local timing profiles, confidence and outlier handling without telemetry.

### M5 — Canonical assets and export normalization

Established validated GLB 2.0 as the production master and added exact GLB, OBJ compatibility ZIP and raw STL export.

### M6 — Print preparation

Added explicit millimetre sizing, orientation, topology analysis, conservative repair, flat-base assistance, 3MF and prepared STL export.

### M7 — Release packaging infrastructure

Implemented the code-side release boundary:

- pinned Python 3.12 standalone runtimes with per-platform SHA-256 verification;
- Tauri resource bundling of that interpreter;
- model installers that prefer the bundled interpreter in packaged builds;
- reproducible npm and Cargo lockfiles;
- active Tauri bundle configuration;
- native macOS/Windows/Linux icon assets derived from the canonical Still2Solid mark;
- release CI for Apple-Silicon macOS, Windows x64 and Linux x64;
- macOS certificate/signing/notarization secret hooks;
- draft GitHub release creation from version tags.

This means the packaging implementation is complete. It does **not** claim that a signed public release has already been produced: that requires configured signing credentials and successful release runs on the supported platforms.

### M8 — Additional production model

Added Stable Fast 3D as an audited second production adapter without weakening the catalogue policy:

- upstream source pinned to immutable GitHub commit `ff21fc491b4dc5314bf6734c7c0dabd86b5f5bb2`;
- gated Hugging Face model pinned to immutable revision `f0c9a8ffd62cb1bbc8a7a53c9f87a0be1b6be778`;
- model checkpoint SHA-256 verification before activation;
- explicit Stability AI Community License acknowledgement in Model Manager;
- Hugging Face access token supplied only to the installer process and not stored by Still2Solid;
- DINO/support assets cached during installation for offline inference;
- one-shot local SF3D worker with cancellation and cleanup;
- canonical GLB result path shared with preview/export/print preparation;
- local timing/ETA learning shared with other production adapters;
- SF3D remains an explicit opt-in and is never selected automatically because its licence is conditional/gated.

TRELLIS.2 remains catalogue-only because its upstream Linux/NVIDIA/VRAM requirements do not fit the primary product target. Models with incompatible regional licensing remain excluded.

## Open validation gate — M1 with 8 GB unified memory

The product target includes low-memory Apple Silicon. The code exposes an explicit TripoSR experimental-install path on 8 GB Apple Silicon, but the recommendation policy remains conservative until a real benchmark is run on the target machine.

Before changing 8 GB from **Memory constrained** to a normal recommendation, capture at least:

- macOS version and exact hardware;
- successful packaged-runtime and model installation;
- Fast, Standard and Best behavior where practical;
- CPU vs MPS behavior;
- peak memory/pressure observations;
- elapsed time by stage;
- cancellation and recovery behavior;
- repeated-run stability;
- representative simple and difficult images;
- whether the 15–30 second product goal is realistic on that hardware.

The result may be “supported but slower than the goal.” The policy should follow measurement rather than optimism.

## Open operational release gate

Before publishing a build as a normal-user release:

- run the release workflow for the intended tag;
- configure and verify macOS signing/notarization credentials;
- define/verify the Windows signing arrangement if signed Windows distribution is required;
- install the produced artifacts on clean supported machines;
- verify native icon/branding, bundled Python discovery, model installation, generation, cancellation, export and uninstall/reinstall behavior;
- generate/review the final dependency/SBOM and third-party licence output from the release artifacts.

These steps validate M7; they are not additional architecture milestones.

## Later product ideas — not committed milestones

Possible post-M8 work includes batch queues, multi-view input where a model genuinely supports it, richer print/slicer hand-off, project history, automatic update UX, additional export/material workflows and opt-in privacy-preserving benchmark sharing.

The product rule remains simple: **image in, local 3D out**. New features should not bury that flow.
