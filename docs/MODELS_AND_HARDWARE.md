# Models & Hardware

Still2Solid treats model choice as a product, licence and reliability decision rather than exposing every model available online.

Model Manager combines three questions:

1. what the computer can plausibly run;
2. what the upstream implementation actually supports;
3. whether the model's licence/gating is appropriate for automatic or explicit use.

A model can therefore be installed and usable without being an automatic recommendation.

## Compatibility labels

| Label | Meaning |
| --- | --- |
| **Recommended** | Best safe automatic choice among reviewed candidates for this hardware. |
| **Compatible** | Hardware/policy checks pass, but another candidate may be preferred. |
| **Compatible · slow path** | Expected to work through a slower backend such as CPU. |
| **Memory constrained** | Possible in some circumstances, but memory pressure is high enough that Still2Solid will not present it as a safe normal choice. |
| **Unsupported** | Platform/backend requirements do not match. |
| **Licence restricted** | Licence/gating prevents normal catalogue use or automatic selection. |

These labels are intentionally conservative. “Compatible” is not a benchmark result and “Memory constrained” is not the same as “impossible.”

## TripoSR

**Role:** primary permissive production adapter.

Why it remains the default:

- official code and pretrained weights are MIT licensed;
- materially lighter than the large alternatives reviewed for this project;
- fits the single-image-to-textured-3D workflow;
- can be installed from immutable source/model revisions with integrity checks;
- does not require `trust_remote_code=True`;
- works with Still2Solid's one-shot worker/canonical-GLB architecture.

The upstream default is commonly described around a ~6 GB VRAM class workload. Still2Solid uses conservative product settings and does not translate that figure into an unsupported promise for unified-memory Macs.

### Apple Silicon

Apple Silicon is detected as unified-memory hardware. Metal/MPS can be exposed as a backend option, but TripoSR upstream does not provide a Still2Solid-quality guarantee for MPS.

Current policy:

- 16 GB+ Apple Silicon can clear the catalogue compatibility threshold with caveats;
- 8 GB Apple Silicon remains **Memory constrained** and is not automatically recommended;
- an explicit experimental install is available so the actual target can be measured.

The 8 GB M1 target-device benchmark remains an open validation gate.

### NVIDIA and CPU

NVIDIA hardware is detected through `nvidia-smi` when available. Less than roughly 6 GB reported VRAM is conservatively treated as memory constrained for TripoSR. CPU remains the fallback path when acceleration is unavailable or inappropriate and may be substantially slower.

## Stable Fast 3D (SF3D)

**Role:** second production adapter, explicit opt-in only.

M8 turns SF3D from a catalogue-only entry into a real local adapter while preserving its licence and hardware restrictions.

Important constraints:

- upstream source is pinned to GitHub commit `ff21fc491b4dc5314bf6734c7c0dabd86b5f5bb2`;
- the gated Hugging Face model is pinned to revision `f0c9a8ffd62cb1bbc8a7a53c9f87a0be1b6be778` and its checkpoint is SHA-256 verified;
- MPS support is experimental;
- upstream guidance recommends CPU below 32 GB unified memory because the MPS path can consume more memory;
- the Stability AI Community License is conditional and the model repository is gated;
- installation therefore requires explicit licence acknowledgement and a user-supplied Hugging Face read token;
- Still2Solid does not store that token;
- SF3D is never an automatic recommendation even when hardware compatibility looks acceptable.

When selected, SF3D uses the same one-shot local generation, progress/cancellation, canonical GLB, preview/export and print-preparation layers as TripoSR.

## TRELLIS.2 4B

TRELLIS.2 remains catalogue-only. The official environment is Linux + NVIDIA with at least about 24 GB VRAM. A suitable workstation can pass the hardware assessment, but this does not create an executable adapter; its runtime/dependency review remains separate from M8.

## Why Hunyuan3D 2.1 is absent

Hunyuan3D 2.1 is intentionally excluded from the official catalogue because its published licence excludes use in the European Union, United Kingdom and South Korea. Still2Solid does not encourage workarounds for regional licence restrictions.

## Production model installation security

A model is not activated merely because files were downloaded. Production installers must:

- use allowlisted model IDs;
- use immutable upstream source and model revisions;
- verify executable/model assets before activation;
- stage installation separately from the active runtime;
- discard incomplete/broken staging state;
- avoid arbitrary downloaded remote code;
- keep credentials out of persistent application state;
- leave inference capable of running offline after installation.

Generation then runs in a one-shot local child process rather than a persistent server.

## Adding a later model

A future adapter must answer all of the following before it becomes executable:

- Is its licence usable in intended regions and use cases?
- Can exact source/model revisions be immutable?
- Can required executable/model assets be integrity-verified?
- Does it require arbitrary remote code?
- Which hardware/backend combinations have actually been tested?
- What is the realistic memory floor?
- Does its output fit the canonical-asset contract?
- Can cancellation and process cleanup be implemented safely?
- Can CI validate integration without downloading multi-gigabyte weights?

See [Model Licence Policy](MODEL_LICENSE_POLICY.md), [M8](M8.md) and [Architecture](ARCHITECTURE.md).
