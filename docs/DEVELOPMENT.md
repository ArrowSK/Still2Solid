# Development

Still2Solid is intentionally split so UI work, model policy, native trust decisions, inference and export/print logic can evolve without becoming one large script.

## Repository map

```text
Still2Solid/
├─ apps/desktop/ui/           Svelte/TypeScript desktop UI
│  ├─ public/brand/           in-app branding assets
│  └─ src/lib/                model, runtime, timing, export and print-prep modules
├─ src-tauri/                 trusted Rust/Tauri core
│  ├─ icons/                  generated native application icons
│  ├─ resources/python/       prepared bundled Python resource (build output, not source)
│  └─ src/                    hardware probe, installers, worker/process bridges
├─ workers/
│  ├─ triposr/                isolated TripoSR worker assets
│  └─ sf3d/                   gated SF3D installer + worker
├─ scripts/                   release/runtime preparation scripts
├─ assets/branding/           canonical repository/readme artwork
├─ docs/                      user, architecture, milestone and policy docs
└─ .github/workflows/         CI and release workflows
```

## Local setup

See [Getting Started](GETTING_STARTED.md) for prerequisites. The short version is:

```bash
npm ci
npm run tauri:dev
```

## Useful scripts

```bash
npm run dev        # browser-only UI preview
npm run check      # Svelte/TypeScript checks
npm run test       # Vitest unit tests
npm run build      # Vite production build
npm run tauri:dev  # native desktop development run
npm run tauri:build
```

Native checks:

```bash
cargo check --locked --manifest-path src-tauri/Cargo.toml
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

Worker/runtime syntax checks:

```bash
python3 -m py_compile \
  workers/triposr/worker.py \
  workers/sf3d/install.py \
  workers/sf3d/worker.py \
  scripts/prepare_python_runtime.py
```

Ordinary CI deliberately does **not** download multi-gigabyte production model weights or run full inference. Expensive model/hardware validation belongs in explicit benchmark work.

## Change philosophy

A working layer should not be replaced merely because another layer has a bug.

Examples:

- a viewer bug should not trigger a runtime redesign;
- an MPS problem should not weaken checksum verification;
- an OBJ limitation should not mutate the canonical GLB;
- a print-repair failure should not silently remesh the production master;
- a background-detection false positive should be fixed in the heuristic/UI rather than making foreground isolation mandatory;
- an SF3D install failure must not disable the working TripoSR or Mock3D paths.

Prefer small, testable changes with an obvious rollback boundary.

## UI architecture

The Svelte UI owns interaction and presentation: file selection/drop, simple/Advanced controls, Background guidance, model/runtime state presentation, progress/ETA, Three.js preview, canonical-asset inspection, derived exports and print-preparation controls.

It does not own trusted model installation decisions or long-lived process management.

### Background check

`backgroundAnalysis.ts` is deliberately not a cloud segmentation service. It downsamples the selected local image and inspects transparency plus edge/centre colour statistics. `BackgroundAdvisor.svelte` turns that result into guidance and binds to the same `backgroundRemoval` setting used by Advanced mode.

Future changes must keep the check local, fast, uncertainty-aware and user-overridable.

## Model adapter contract

Model-specific generation logic is hidden behind a common adapter interface. Production adapters currently include TripoSR and optional Stable Fast 3D. Each adapter provides manifest/stage metadata, generation input, progress events, cancellation and structured results/errors. Installation/runtime state remains a separate concern.

## Trusted Rust boundary

The Rust/Tauri core owns operations that should not be delegated to downloaded model code:

- hardware probing;
- allowlisted runtime installation;
- pinned source/model downloads;
- checksum/hash verification;
- staging and activation;
- child-process lifecycle;
- cancellation/cleanup;
- application-local filesystem paths.

Prefer narrow commands over generic shell/process/filesystem capability in the UI.

## Production worker boundary

Production workers are one-shot processes. They must receive a bounded job description, use installed/verified local assets, avoid model/code downloads during inference, emit structured progress/results, be killable by the trusted host and exit after each job.

Do not replace this with a persistent localhost FastAPI/Flask model server for convenience.

## Model/runtime changes

Before changing a production pin:

1. review the upstream licence again;
2. record an immutable source/model revision;
3. calculate/verify required integrity hashes;
4. review new dependencies and their licences;
5. confirm arbitrary remote code is not required;
6. test staging failure and cleanup;
7. update third-party notices, model docs and deterministic tests.

For conditional/gated models, also verify that user acceptance remains explicit and credentials are not persisted.

See [Model Licence Policy](MODEL_LICENSE_POLICY.md).

## Bundled Python runtime

M7 prepares platform-specific Python 3.12 standalone artifacts from the immutable metadata in `scripts/python-runtime.json`. `scripts/prepare_python_runtime.py` verifies SHA-256 before extraction into Tauri resources.

Do not commit the extracted runtime tree. Packaged builds carry it as a generated resource. Model installers then create their own private environments from that verified base interpreter.

Changing a Python runtime artifact requires updating its filename/checksum metadata and re-running the bundled-runtime CI check.

## Timing and ETA

Timing profiles remain local and are keyed by hardware identity, model/version, quality, backend and foreground-isolation setting. Only successful runs are recorded; failed/cancelled jobs must not train the profile. Stored history remains bounded and does not contain source-image content.

## Canonical assets and export

The validated production GLB is the canonical master. Derived exports are non-destructive: exact GLB, OBJ package, raw STL, a separate Print Prep geometry copy, and 3MF/prepared STL from that prepared copy.

Never make a lossy export the new internal master.

## Print preparation

Print repair is deliberately conservative. Future repair algorithms should prefer “repair incomplete” over fabricating major missing geometry. Tests should cover topology classification, winding, degenerates, hole handling, scaling/orientation, 3MF structure and STL layout.

## CI

The main workflow validates:

- `npm ci`, Svelte/TypeScript checks, Vitest and Vite build;
- Python syntax for both production workers/installers and bundled-runtime preparation;
- locked Rust check/tests on Apple-Silicon macOS CI;
- download/checksum/interpreter validation for one pinned Linux bundled Python artifact.

Release CI separately prepares the target Python runtime and invokes Tauri bundling for Apple-Silicon macOS, Windows x64 and Linux x64.

## Pull-request checklist

Before merging a release-bound change:

- [ ] Existing working behavior is preserved unless intentionally changed.
- [ ] User-facing wording matches implemented behavior.
- [ ] Security/privacy boundaries are not weakened.
- [ ] Model/licence metadata is current where relevant.
- [ ] Lockfiles remain synchronized.
- [ ] `npm run check`, tests and build pass.
- [ ] Locked Rust check/tests pass.
- [ ] Worker/runtime syntax checks pass.
- [ ] Documentation describes implemented state rather than a future plan.

## Releases

M7 packaging infrastructure is implemented. Do not nevertheless call a build a signed public release until the release workflow has succeeded for the intended tag, required signing/notarization credentials have been supplied, produced installers have been tested on clean target systems and the release SBOM/licence review has been completed.

See [M7](M7.md), [M8](M8.md) and [Roadmap](ROADMAP.md).
