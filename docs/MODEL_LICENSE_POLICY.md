# Model licence policy

Still2Solid application code is Apache-2.0. Model weights, model-specific source and runtime components keep their own licences and must never be assumed to inherit the application licence.

## Registry states

Every production model is classified as one of:

- `verified-permissive`
- `conditional`
- `restricted`
- `unknown`

A production adapter may be enabled only after source-code licence, pretrained-weight licence, regional/commercial conditions, immutable revision strategy and required notices have been reviewed and recorded.

## Executable production models

### TripoSR — `verified-permissive`

- Upstream source: `VAST-AI-Research/TripoSR`, pinned to commit `107cefdc244c39106fa830359024f6a2f1c78871`.
- Model repository: `stabilityai/TripoSR`, pinned to revision `5b521936b01fbe1890f6f9baed0254ab6351c04a`.
- Upstream states that source code and pretrained model are MIT licensed.
- Downloaded source files are verified against pinned Git blob hashes.
- `model.ckpt` is verified with SHA-256 `429e2c6b22a0923967459de24d67f05962b235f79cde6b032aa7ed2ffcd970ee` before activation.
- The upstream MIT licence is stored with the installed source.

TripoSR remains the permissive automatic production candidate where the hardware policy considers it safe.

### Stable Fast 3D — `conditional`

M8 adds Stable Fast 3D as an executable **explicit opt-in** model.

- Upstream source: `Stability-AI/stable-fast-3d`, pinned to GitHub commit `ff21fc491b4dc5314bf6734c7c0dabd86b5f5bb2`.
- Gated model repository: `stabilityai/stable-fast-3d`, pinned to Hugging Face revision `f0c9a8ffd62cb1bbc8a7a53c9f87a0be1b6be778`.
- `model.safetensors` is verified with SHA-256 `a3416e1cf654e7d4f5e75f116cec2c3f0a14501a77d30c2f6068bbda178de388` before activation.
- The model uses the Stability AI Community License. The current upstream model card states that commercial use by individuals/organizations with annual revenue above US$1M requires an enterprise commercial licence; users remain responsible for the terms applicable to their use.
- Installation requires the user to have accepted the upstream gate and to explicitly acknowledge the model licence in Still2Solid.
- The user supplies a Hugging Face read token for installation. Still2Solid passes it to the installer process only and does not store it in application settings or the install manifest.
- Hardware compatibility does not imply licence eligibility.
- SF3D is never silently auto-selected; choosing it is an explicit user action.

The source and model revisions are immutable identifiers. Changing either revision requires a fresh licence/security review and updated integrity evidence.

## Foreground isolation

Optional foreground isolation uses rembg under MIT and its U2Net model asset. The original U-2-Net project is Apache-2.0 licensed. The downloaded ONNX file is checksum-verified before a production installation is activated.

## Catalogue-only model

**TRELLIS.2 4B** remains catalogue-only. Its model/main code is MIT, but its current upstream Linux + NVIDIA ≥24 GB VRAM requirements do not fit the primary target and its full runtime dependency set must still be reviewed before any future executable adapter.

Models whose licence excludes the intended distribution region are not included as official executable options.

## Runtime and release safeguards

- Keep AI weights out of Git.
- Use immutable source/model revisions rather than moving branches.
- Verify downloaded production assets before activation.
- Never enable arbitrary remote model code such as `trust_remote_code=True` as a convenience shortcut.
- Keep inference offline after installation and avoid persistent localhost model servers.
- Re-check upstream licence metadata whenever a pinned revision changes.
- Conditional/community licences require explicit disclosure and user acceptance before installation.
- Never accept gated-model terms on the user's behalf.
- Generate/review a complete SBOM and dependency-licence report from final release artifacts before publishing a signed release.

Version pinning alone does not change third-party licence terms.
