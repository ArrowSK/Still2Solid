# Security & Privacy

Still2Solid is local-first by design. Privacy and supply-chain controls are architectural constraints, not optional presentation features.

## What stays on the computer

During normal generation:

- the selected source image;
- the small downscaled sample used by Background check;
- optional foreground-isolation processing;
- model inference;
- generated geometry/textures;
- preview assets;
- learned timing history;
- export conversion;
- print-preparation analysis and repair.

Still2Solid does not include telemetry or analytics.

## When network access is used

Network access is used when preparing release/runtime dependencies and when the user installs a production model. Production inference itself is designed to work from local installed assets.

Downloads are constrained by the adapter/release code:

- model IDs are allowlisted;
- production source/model revisions are immutable;
- required source/model/runtime assets are integrity-checked;
- incomplete or unverified staging state is not activated;
- inference sets supported upstream libraries to offline mode so a worker cannot silently fetch a replacement model during a job.

## Bundled Python runtime

M7 release builds prepare a pinned Python 3.12 standalone artifact selected by target platform. Its filename and SHA-256 are stored in `scripts/python-runtime.json`, and the archive is verified before extraction into Tauri resources.

Model installers use that bundled interpreter as the normal packaged-build base and create separate private model environments. Source/development builds can use an explicit compatible developer interpreter, but ordinary packaged users should not need to configure Python.

## Stable Fast 3D gated credentials

SF3D is optional and gated. Still2Solid never accepts the upstream gate on the user's behalf.

When the user explicitly installs SF3D:

- the user enters a Hugging Face read token into Model Manager;
- the token is passed only to the installer invocation/process;
- the Python installer removes the token from its own environment as soon as it reads it;
- the token is not written to Still2Solid settings or the model install manifest;
- the manifest explicitly records `tokenStored: false`;
- installation still requires integrity verification of the pinned model checkpoint before activation.

The user remains responsible for the upstream model licence/gating terms applicable to their use.

## No localhost inference server

Still2Solid does not run a persistent FastAPI/Flask/HTTP inference endpoint. Production inference is launched as a one-shot child process, communicates through the controlled host/process path and exits when the job finishes or is cancelled.

This reduces open local ports, persistent model memory, reconnect/background-loop complexity and long-lived model process state.

## No arbitrary remote model code

Still2Solid does not enable `trust_remote_code=True` for production model loading. A model that requires arbitrary downloaded Python code must not become a production adapter without a separate architectural/security decision.

## Trusted native boundary

The Tauri/Rust core owns hardware probing, installer allowlists, runtime paths, verification, child-process lifecycle and cancellation/cleanup. The Svelte UI is not given generic shell execution or unrestricted filesystem/process capability.

## Background check privacy

Background check downsamples the already-selected image inside the webview and evaluates transparency plus simple edge/centre colour statistics. It does not upload the image, call an external vision API, persist pixel samples or identify the object/person in the image. Its result is only guidance for the foreground-isolation toggle.

## Learned timing data

Timing profiles are local and contain only technical execution context such as model/hardware/backend/quality, stage durations, total duration, timestamp and acceptance/exclusion state. They do not store the source image, source filename or generated asset content.

## Generated files

Exports are created locally. Still2Solid does not upload GLB/OBJ/STL/3MF data during export. Derived export/print-preparation paths do not silently rewrite the canonical GLB master.

## Storage cleanup and uninstall privacy

Downloaded production models live in Still2Solid's application-data area rather than inside the application bundle. This keeps updates/reinstalls separate from multi-gigabyte model downloads.

**Settings → Storage** exposes only app-scoped cleanup operations. The Rust side resolves Tauri application data/cache/config/local-data roots, validates that cleanup targets are Still2Solid-owned paths, and refuses unexpected paths before recursive deletion. The UI does not receive generic filesystem deletion capability.

**Prepare for uninstall** first invokes the existing model uninstall paths, then removes Still2Solid-owned app data/cache and local `still2solid.*` preferences. User-exported model files saved outside Still2Solid's application-data directories are not part of cleanup.

## Content Security Policy

The desktop webview uses an explicit Content Security Policy in Tauri configuration. Changes that broaden `script-src`, `connect-src` or other directives are security changes and require justification. Do not add `unsafe-eval`, broad remote script origins or arbitrary network endpoints simply to make a dependency convenient.

## Supply-chain surfaces

There are three separately reviewable surfaces:

1. application npm/Rust dependencies;
2. bundled Python/release runtime artifacts;
3. model-specific source, weights and Python dependencies.

Model weights do not inherit the Apache-2.0 application licence. Conditional model terms also do not become permissive merely because the adapter code is open source.

See [Model Licence Policy](MODEL_LICENSE_POLICY.md), [M7](M7.md), [M8](M8.md) and `THIRD_PARTY_NOTICES.md`.

## Security invariants

Unless an explicit architecture decision changes them, the following remain true:

- no cloud inference in the core product flow;
- no telemetry/analytics;
- no persistent localhost inference service;
- no arbitrary `trust_remote_code` model execution;
- production and bundled-runtime downloads are pinned/verified before activation/use;
- gated-model credentials are not persisted by Still2Solid;
- source images are not uploaded for background classification;
- failed verification never silently falls back to unverified execution;
- cancellation terminates/cleans up the worker;
- derived exports never mutate the canonical production master.

## Reporting a security issue

Please follow repository-level [SECURITY.md](../SECURITY.md). Avoid publishing actionable exploit details in a normal public issue before a fix is available.
