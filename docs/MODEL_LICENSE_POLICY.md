# Model licence policy

Still2Solid application code is Apache-2.0. Model weights and model-specific code are separately licensed assets and must never be assumed to inherit the application licence.

## Registry states

Every production model must be classified as one of:

- `verified-permissive`
- `conditional`
- `restricted`
- `unknown`

A model may only be enabled in the official catalogue after its source-code licence, pretrained-weight licence, geographic restrictions, commercial-use conditions and required notices have been reviewed and recorded.

## M1

Mock3D contains no external model weights and therefore has `not-applicable` model-licence status.

## M2 catalogue review

M2 records catalogue metadata but does not redistribute or install any production weights.

- **TripoSR** — `verified-permissive`; upstream repository and model card identify the code and pretrained model as MIT licensed.
- **Stable Fast 3D** — `conditional`; gated access under the Stability AI Community License. Hardware compatibility does not imply licence eligibility, and the model cannot be auto-selected.
- **TRELLIS.2 4B** — `verified-permissive` for the model and main code under MIT; separately licensed runtime dependencies still require review before a distributable worker can be enabled.

Catalogue status is evidence for product policy, not a substitute for re-checking the pinned upstream licence at install time.

## Required safeguards for M3 and later

- Pin model repositories and weights to immutable revisions.
- Record source URL, revision, SHA-256 and licence text/revision.
- Never enable `trust_remote_code=True` automatically.
- Treat a downloadable model as `unknown` until its rights are verified.
- Keep model weights out of Git.
- Conditional/community licences require explicit UI disclosure before installation.
- Never accept gated-model terms on the user's behalf.
- Re-check licence metadata when a model revision changes.
