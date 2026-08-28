# Roadmap

Still2Solid uses milestones as capability boundaries, not as marketing version names. A milestone is only “done” when the code, tests and user-facing behavior for that boundary exist. Hardware claims still require real hardware validation.

## Where the project is now

The application has reached the end of M6 functionality:

- a working desktop shell;
- hardware-aware model policy;
- a real local TripoSR production path;
- learned local progress/ETA;
- canonical GLB + derived exports;
- non-destructive print preparation and 3MF;
- local background guidance;
- project branding and a complete documentation set.

It is **not yet an end-user release** because the runtime packaging/signing/notarization work in M7 remains open.

## Completed milestones

### M1 — Product shell and Mock3D

Delivered the simple image-first desktop workflow, quality controls, deterministic Mock3D adapter, progress/cancel flow and interactive preview.

### M2 — Model Manager and hardware policy

Added native hardware probing, curated model catalogue, deterministic compatibility states and conservative automatic recommendation rules.

### M3 — Production TripoSR adapter

Added the first real production model path:

- pinned and verified install;
- private Python environment;
- one-shot worker;
- foreground isolation;
- backend choice;
- production GLB generation;
- safe Mock3D fallback.

### M4 — Learned progress and ETA

Added per-machine/per-setting local timing profiles, confidence and outlier handling without telemetry.

### M5 — Canonical assets and export normalization

Established validated GLB 2.0 as the production master and added exact GLB, OBJ compatibility ZIP and raw STL export.

### M6 — Print preparation

Added explicit millimetre sizing, orientation, topology analysis, conservative repair, flat-base assistance, 3MF and prepared STL export.

### 0.6.1 product polish

This is not a new architecture milestone. It closes important usability/repository-quality gaps after M6:

- final Still2Solid branding in the application/repository;
- local Background check with a clear user-controlled foreground-isolation recommendation;
- humane README and user/developer/troubleshooting/security/model/branding documentation;
- release-state and roadmap wording that distinguishes implemented capability from unvalidated hardware claims.

## Open validation gate — M1 with 8 GB unified memory

The product target includes low-memory Apple Silicon. The code supports an explicit TripoSR experimental-install path on 8 GB Apple Silicon, but the recommendation policy remains conservative until a real benchmark is run on the target machine.

Before changing 8 GB from **Memory constrained** to a normal recommendation, capture at least:

- macOS version and exact hardware;
- successful install/runtime verification;
- Fast, Standard and Best behavior where practical;
- CPU vs MPS behavior;
- peak memory/pressure observations;
- elapsed time by stage;
- cancellation and recovery behavior;
- repeated-run stability;
- representative simple and difficult images;
- whether the 15–30 second product goal is realistic on that hardware.

The result may be “supported but slower than the goal.” That is acceptable. The policy should reflect measurement rather than optimism.

## M7 — Release packaging

Goal: make Still2Solid installable by a normal user without asking them to build a Rust/Svelte app or prepare Python manually.

Planned scope:

- bundled/versioned production Python runtime;
- reproducible runtime layout;
- application bundle activation in Tauri;
- correct platform icon assets;
- macOS signing and notarization flow;
- Windows signing strategy;
- release CI/artifacts;
- clean install/uninstall behavior;
- app-data/runtime-data separation;
- model/runtime upgrade path;
- first-run error messages that do not require terminal knowledge;
- documented rollback/recovery behavior;
- release checks on at least the primary Apple Silicon target and one additional supported platform.

M7 should not silently move inference to a cloud service to simplify packaging.

## M8 — Additional models

Goal: make the model architecture genuinely useful beyond the first adapter without turning Model Manager into an unsafe catalogue of everything available online.

A model is eligible only after:

- licence/region review;
- hardware floor and backend review;
- immutable source/model pin strategy;
- checksum/verification strategy;
- no arbitrary remote-code requirement;
- output compatibility with the canonical asset layer;
- cancellation/cleanup design;
- CI-test strategy that does not require downloading the full model.

Likely categories to explore:

- a stronger high-memory workstation model;
- a genuinely Apple-Silicon-friendly alternative if one is available under suitable terms;
- optional specialized geometry/texturing models where they improve a clear product workflow.

Conditional/gated models should remain explicit opt-ins rather than automatic defaults.

## Later product ideas — not committed milestones

These ideas may make sense after M7/M8, but they are intentionally not presented as promised features:

- batch image queue;
- multi-view input where a model genuinely supports it;
- richer print tools such as supports or slicer hand-off;
- project/history library;
- automatic update UI;
- additional export/material workflows;
- benchmark sharing that is opt-in and privacy-preserving.

The core product should remain understandable: **image in, local 3D out**. New features should not bury that flow.

## Definition of “production-ready” for this project

Before calling the desktop application production-ready for ordinary users, Still2Solid should have:

- a packaged runtime with no manual Python requirement;
- signed/notarized release artifacts where the platform supports it;
- verified application icons/branding in packaged builds;
- a reproducible release workflow;
- hardware validation for the machines the UI recommends automatically;
- clean first-run installation and model installation;
- clear failure recovery without terminal-only instructions;
- green automated checks;
- current third-party/licence documentation;
- no regression in the local-first/security invariants.

That is the M7 release bar.
