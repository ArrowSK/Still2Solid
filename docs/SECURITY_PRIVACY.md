# Security & Privacy

Still2Solid is local-first by design. That means privacy is not a marketing label added after the architecture; it is one of the constraints used to decide how the application is built.

## What stays on the computer

During normal generation:

- the selected source image;
- the small downscaled image used by Background check;
- optional foreground-isolation processing;
- model inference;
- generated geometry/textures;
- preview assets;
- learned timing history;
- export conversion;
- print-preparation analysis and repair.

Still2Solid does not include telemetry or analytics.

## When network access is used

The current production runtime needs network access during installation because Still2Solid has to retrieve the reviewed runtime/source/model assets.

Those downloads are not arbitrary:

- the production model ID is allowlisted;
- upstream source/model revisions are pinned;
- downloaded source files are verified against expected hashes;
- the model checkpoint is verified with SHA-256;
- the foreground-removal asset is verified;
- incomplete/unverified staging state is not activated.

Once installed, the inference worker is designed to use the local runtime and local weights. Runtime model downloads are blocked during generation.

## No localhost inference server

Still2Solid does not run a persistent FastAPI/Flask/HTTP inference endpoint on the user's machine.

Production inference is launched as a one-shot child process for the job, communicates through the controlled process/IPC path, and exits when the job finishes or is cancelled.

This reduces:

- open local ports;
- accidental network exposure;
- persistent model memory use;
- reconnect/background-loop complexity;
- the amount of long-lived process state.

## No arbitrary remote model code

Still2Solid does not enable `trust_remote_code=True` for production model loading.

A model that requires arbitrary downloaded Python code should not be made a production adapter without a separate design/security review. “It works on Hugging Face” is not enough to expand the trusted code boundary.

## Trusted native boundary

The Tauri/Rust core is the trusted native layer. It owns hardware probing, installer allowlists, download verification, runtime paths and worker lifecycle.

The Svelte UI should not be given generic shell execution or unrestricted filesystem/process capability simply because it would make development easier.

## Background check privacy

The Background check is intentionally small and local. It downsamples the already-selected image in the webview and evaluates transparency plus simple edge/centre colour statistics.

It does not:

- upload the image;
- call an external vision API;
- persist the pixel sample;
- identify the object;
- infer personal information from the image.

Its result is only guidance for the existing foreground-isolation toggle.

## Learned timing data

M4 timing profiles are stored locally and are intended to contain only technical timing context, for example:

- hardware/model/backend/quality context;
- stage durations;
- total duration;
- completion timestamp;
- acceptance/exclusion state.

They must not store the source image, its filename or generated asset content.

## Generated files

Exports are created locally. Still2Solid does not upload GLB/OBJ/STL/3MF data as part of the export flow.

The GLB master is not silently rewritten by compatibility or print-preparation exports.

## Content Security Policy

The desktop webview uses an explicit Content Security Policy in the Tauri configuration. Changes that broaden `script-src`, `connect-src` or other CSP directives should be treated as security changes and justified in review.

Do not add `'unsafe-eval'`, broad remote script origins or arbitrary network endpoints to make a library work without first understanding why they are needed.

## Dependency/model supply chain

There are two separate supply-chain surfaces:

1. application dependencies (npm/Rust/Python runtime packages);
2. model/runtime assets.

They should remain separately reviewable. Model weights do not inherit the Apache-2.0 application licence and should not be treated like ordinary source dependencies.

See [Model Licence Policy](MODEL_LICENSE_POLICY.md) and `THIRD_PARTY_NOTICES.md`.

## Security invariants

The following are intended to remain true unless there is an explicit architectural decision to change them:

- no cloud inference in the core product flow;
- no telemetry/analytics;
- no persistent localhost inference service;
- no arbitrary `trust_remote_code` model execution;
- production downloads are pinned/verified before activation;
- source images are not uploaded for background classification;
- failed verification never silently falls back to unverified execution;
- cancellation must terminate/clean up the worker;
- derived exports never mutate the canonical production master.

## Reporting a security issue

Please follow the repository-level [SECURITY.md](../SECURITY.md). Avoid publishing exploit details in a normal public issue while a vulnerability is still actionable.
