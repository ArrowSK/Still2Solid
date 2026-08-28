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

## Required safeguards for later milestones

- Pin model repositories and weights to immutable revisions.
- Record source URL, revision, SHA-256 and licence text/revision.
- Never enable `trust_remote_code=True` automatically.
- Treat a downloadable model as `unknown` until its rights are verified.
- Keep model weights out of Git.
- Conditional/community licences require explicit UI disclosure before installation.
