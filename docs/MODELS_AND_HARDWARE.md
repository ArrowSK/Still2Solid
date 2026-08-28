# Models & Hardware

Still2Solid treats model choice as a product and safety decision, not a dropdown of every model that can be found online.

The Model Manager combines three things:

1. what the computer can plausibly run;
2. what the upstream model actually supports;
3. whether the model's licence is appropriate for the catalogue and intended use.

A model can therefore be visible without being automatically recommended.

## Compatibility labels

| Label | Meaning |
| --- | --- |
| **Recommended** | Best safe automatic choice among the currently reviewed candidates for this hardware. |
| **Compatible** | Hardware/policy checks pass, but another candidate may be preferred. |
| **Compatible · slow path** | Expected to work through a slower backend such as CPU. |
| **Memory constrained** | Technically possible in some circumstances, but memory pressure is high enough that Still2Solid will not present it as a safe normal choice. |
| **Unsupported** | Platform/backend requirements do not match. |
| **Licence restricted** | Licence/gating prevents normal catalogue use or automatic selection. |

These labels are intentionally conservative. “Compatible” is not a benchmark result and “Memory constrained” is not the same as “impossible.”

## TripoSR

**Role:** first production adapter.

Why it was selected:

- official code and pretrained weights are MIT licensed;
- materially lighter than the large modern alternatives reviewed for this project;
- supports the single-image-to-3D workflow Still2Solid needs;
- can produce textured output;
- its runtime can be isolated and pinned without `trust_remote_code=True`.

The upstream default is commonly described around a ~6 GB VRAM class workload. Still2Solid uses more conservative product presets than upstream defaults to reduce pressure on consumer hardware.

### Apple Silicon

Apple Silicon is detected as unified-memory hardware. Still2Solid exposes Metal/MPS as a backend option where the runtime supports it, but **TripoSR upstream does not provide a Still2Solid-quality guarantee for MPS**. Backend support must be validated empirically.

Current policy:

- 16 GB+ Apple Silicon can clear the catalogue compatibility threshold with caveats;
- 8 GB Apple Silicon is **Memory constrained** and is not automatically recommended;
- an explicit experimental install can still be offered so the path can be benchmarked rather than artificially blocked.

The actual 8 GB M1 target-device benchmark remains an open validation gate before release policy is relaxed.

### NVIDIA

NVIDIA hardware is detected through `nvidia-smi` when available. CUDA is the most natural acceleration path for TripoSR. Less than roughly 6 GB reported VRAM is conservatively treated as memory constrained by the current policy.

### CPU

CPU is the fallback path when acceleration is unavailable or inappropriate. It can be significantly slower and is labelled accordingly.

## Stable Fast 3D (SF3D)

Still2Solid keeps SF3D as a reviewed catalogue candidate rather than a normal automatic choice.

Important upstream constraints reflected in the catalogue:

- MPS support is experimental;
- upstream testing cited high-memory Apple hardware;
- upstream guidance recommends CPU below 32 GB unified memory because its MPS path can use more memory;
- the Stability AI Community License is conditional and the model is gated.

Because of the licence/gating status, Still2Solid does **not** silently auto-select SF3D even when hardware compatibility looks acceptable.

## TRELLIS.2 4B

TRELLIS.2 is reviewed as a powerful but heavy candidate.

The official environment is Linux + NVIDIA with at least about 24 GB VRAM. That makes it unsuitable for the low-memory Apple Silicon product target. A suitable Linux/NVIDIA workstation can pass the hardware assessment, but runtime dependencies still require separate licence review before production integration.

## Why Hunyuan3D 2.1 is absent

Hunyuan3D 2.1 is intentionally excluded from the official catalogue because its published licence excludes use in the European Union, United Kingdom and South Korea. Still2Solid does not encourage users to work around a regional licence restriction.

## Model installation security

A production model is not activated just because a folder exists.

The TripoSR installer:

- uses an allowlisted model ID;
- pins the upstream source revision;
- pins the model revision;
- verifies source files against Git blob hashes;
- verifies the checkpoint with SHA-256;
- verifies the foreground-removal asset;
- stages installation before activation;
- removes/abandons incomplete staging state when verification fails;
- never enables arbitrary remote code execution.

Generation then runs in a one-shot local child process rather than a permanent server.

## Adding another model

A future model should not be added to the production catalogue until all of the following are answered:

- Is the licence usable in the target regions and for the intended commercial/non-commercial use?
- Can the exact source/model revision be pinned?
- Can every required executable/model asset be verified?
- Does it require `trust_remote_code` or arbitrary downloaded code?
- What hardware/backend combination has actually been tested?
- What is the memory floor?
- Does it return a format that fits the canonical-asset contract?
- Can cancellation and process cleanup be implemented safely?
- Can CI test the adapter without downloading multi-gigabyte weights?

See [Model Licence Policy](MODEL_LICENSE_POLICY.md) and [Architecture](ARCHITECTURE.md).
