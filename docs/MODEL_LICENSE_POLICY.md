# Model licence policy

Still2Solid application code is Apache-2.0. Model weights and model-specific code are separately licensed assets and must never be assumed to inherit the application licence.

## Registry states

Every production model is classified as one of:

- `verified-permissive`
- `conditional`
- `restricted`
- `unknown`

A production adapter may be enabled only after source-code licence, pretrained-weight licence, commercial-use conditions and required notices have been reviewed and recorded.

## M3 executable model

### TripoSR — `verified-permissive`

M3 is intentionally limited to one executable production model.

- Upstream source: `VAST-AI-Research/TripoSR`, pinned to commit `107cefdc244c39106fa830359024f6a2f1c78871`.
- Model repository: `stabilityai/TripoSR`, pinned to revision `5b521936b01fbe1890f6f9baed0254ab6351c04a`.
- Upstream states that the source code and pretrained model are MIT licensed.
- Downloaded source files are verified against pinned Git blob hashes.
- `model.ckpt` is verified with SHA-256 `429e2c6b22a0923967459de24d67f05962b235f79cde6b032aa7ed2ffcd970ee` before activation.
- The upstream MIT licence is downloaded and stored with the installed source.

M3 never uses `trust_remote_code=True` and blocks unexpected Hugging Face downloads during inference.

### Foreground isolation

The optional foreground stage uses rembg under MIT and its U2Net model asset. The original U-2-Net project is Apache-2.0 licensed. The downloaded ONNX file is checked against the checksum published by rembg before the TripoSR installation is activated.

## Catalogue-only models

- **Stable Fast 3D** — `conditional`; gated access under the Stability AI Community License. Hardware compatibility does not imply licence eligibility. M3 does not install or execute it.
- **TRELLIS.2 4B** — `verified-permissive` for the model/main code under MIT, but M3 does not install or execute it and its full runtime dependency set still requires release review.

## Runtime dependencies

The isolated TripoSR environment uses exact package versions. Their licences remain their own; important runtime components are listed in `THIRD_PARTY_NOTICES.md`. Exact package-version pinning does not replace a release SBOM or wheel-hash lock, which remains required before distributable release packaging.

## Safeguards for later milestones

- Keep model weights out of Git.
- Re-check upstream licence metadata whenever a pinned revision changes.
- Generate a complete SBOM and dependency-licence report for release builds.
- Lock distributable Python wheels/artifacts by platform and SHA-256 before signed releases.
- Conditional/community licences require explicit disclosure and acceptance before installation.
- Never accept gated-model terms on the user's behalf.
