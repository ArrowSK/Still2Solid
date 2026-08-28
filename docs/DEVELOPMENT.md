# Development

Still2Solid is intentionally split so UI work, model policy, native trust decisions, inference and export/print logic can evolve without turning into one large script.

## Repository map

```text
Still2Solid/
├─ apps/desktop/ui/           Svelte/TypeScript desktop UI
│  ├─ public/brand/           in-app branding assets
│  └─ src/lib/                model, runtime, timing, export and print-prep modules
├─ src-tauri/                 trusted Rust/Tauri core
│  ├─ icons/                  desktop application icon
│  └─ src/                    hardware probe, installer, worker/process bridge
├─ workers/                   isolated production model worker(s)
├─ assets/branding/           repository/readme brand artwork
├─ docs/                      user, architecture, milestone and policy docs
└─ .github/workflows/         CI
```

## Local setup

See [Getting Started](GETTING_STARTED.md) for prerequisites. The short version is:

```bash
npm install
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
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Worker syntax check:

```bash
python3 -m py_compile workers/triposr_worker.py
```

CI deliberately does **not** download multi-gigabyte production weights or run full inference. Expensive hardware/model validation belongs in explicit benchmark work, not ordinary pull-request CI.

## Change philosophy

A working layer should not be replaced merely because another layer has a bug.

Examples:

- a viewer bug should not trigger a runtime redesign;
- an MPS problem should not weaken checksum verification;
- an OBJ conversion limitation should not mutate the canonical GLB;
- a print-repair failure should not silently remesh the production master;
- a background-detection false positive should be fixed in the heuristic/UI, not by making foreground isolation mandatory.

Prefer small, testable changes with an obvious rollback boundary.

## UI architecture

The Svelte UI owns interaction and presentation:

- file selection/drop;
- simple vs Advanced controls;
- Background check guidance;
- model/runtimes status presentation;
- generation progress/ETA presentation;
- Three.js preview;
- canonical-asset inspection;
- derived export UI;
- print-preparation controls.

It does not own trusted model installation decisions or long-lived native process management.

### Background check

`backgroundAnalysis.ts` is deliberately not an AI segmentation service. It downsamples the selected local image and inspects transparency plus edge/centre colour statistics.

`BackgroundAdvisor.svelte` turns that result into humane guidance and binds to the same `backgroundRemoval` generation setting used by Advanced mode.

Rules for future changes:

- never upload the image just to classify the background;
- keep the check fast enough to feel instantaneous;
- make uncertainty visible;
- never prevent the user from overriding the suggestion;
- add pure pixel-level tests for heuristic changes.

## Model adapter contract

Model-specific generation logic is hidden behind a common adapter interface. The UI should not need model-specific branches for every inference detail.

A production adapter should provide:

- manifest/stage metadata;
- generation input contract;
- progress events;
- cancellation;
- structured result/error information.

Model installation/runtime state remains a separate concern.

## Trusted Rust boundary

The Rust/Tauri core owns operations that should not be casually delegated to downloaded model code:

- hardware probing;
- allowlisted runtime installation;
- pinned source/model downloads;
- checksum/hash verification;
- staging and activation;
- child-process lifecycle;
- cancellation/cleanup;
- app-local filesystem paths.

When expanding this boundary, prefer an explicit narrow command over exposing generic shell/process/filesystem capability to the UI.

## Production worker boundary

The TripoSR worker is a one-shot process. It should:

1. receive a bounded job description;
2. use only installed/pinned local runtime assets;
3. avoid model/code downloads during inference;
4. emit structured progress/results;
5. exit after the job;
6. be killable by the trusted host process.

Do not replace this with a persistent localhost FastAPI/Flask server for convenience.

## Model/runtime changes

Before changing a production pin:

1. review the upstream licence again;
2. record the exact source/model revision;
3. calculate/verify immutable hashes;
4. review new dependencies and their licences;
5. confirm `trust_remote_code` is not required;
6. test staging failure and cleanup;
7. update `THIRD_PARTY_NOTICES.md`, model docs and tests as required.

See [Model Licence Policy](MODEL_LICENSE_POLICY.md).

## Timing and ETA

M4 timing profiles are local and keyed by the conditions that materially affect runtime:

- hardware identity;
- model/version;
- quality;
- backend;
- foreground-isolation setting.

Only successful runs are recorded. Failed/cancelled jobs must not train the profile. Keep the stored history bounded and free of source-image names/content.

## Canonical assets and export

The validated production GLB is the canonical master.

Derived exports should be pure/non-destructive transformations:

- GLB: exact bytes;
- OBJ package: compatibility conversion;
- raw STL: geometry-only conversion;
- Print Prep: separate geometry copy;
- 3MF/prepared STL: derived from the prepared copy.

Never make a lossy export the new internal master.

## Print preparation

Print repair is deliberately conservative. Any future repair algorithm should prefer “repair incomplete” over fabricating major missing geometry.

Tests should cover at least:

- topology classification;
- winding consistency;
- degenerates;
- hole handling;
- scaling/orientation;
- 3MF structure;
- STL binary layout.

## Tests

Good tests focus on deterministic logic:

- compatibility/recommendation policy;
- timing-profile statistics;
- GLB header/asset helpers;
- background heuristic;
- print topology/repair/export structures;
- Rust parsing/install state logic.

Do not make ordinary CI depend on a GPU, a model download or a particular external model host being online.

## Pull-request checklist

Before opening/merging a PR:

- [ ] The change has one clear purpose.
- [ ] Existing working behavior is preserved unless intentionally changed.
- [ ] User-facing wording is understandable without reading source code.
- [ ] Security/privacy boundaries are not weakened.
- [ ] Model/licence metadata is updated if relevant.
- [ ] Tests cover deterministic new logic.
- [ ] `npm run check` passes.
- [ ] `npm run test` passes.
- [ ] `npm run build` passes.
- [ ] Rust check/tests pass when Rust changed — and preferably for every release-bound PR.
- [ ] Worker syntax validation passes when worker code changed.
- [ ] Documentation matches what is actually implemented, not what is merely planned.

## Releases

The repository is currently pre-M7 from an end-user packaging perspective. Do not present an unsigned development build as a polished installer release.

M7 should establish the reproducible package/sign/notarize/release workflow and remove the ordinary end-user dependency on a separately installed Python runtime.
