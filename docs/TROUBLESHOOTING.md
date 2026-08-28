# Troubleshooting

This page starts with the least destructive checks. Still2Solid is deliberately built so a model/runtime problem should not require redesigning or reinstalling the whole application.

## The app shows Mock3D instead of TripoSR

This is usually a runtime/policy state, not a UI failure.

1. Open **Models**.
2. Find TripoSR.
3. Read its compatibility reason.
4. Check whether the runtime says installed and verified.
5. Check whether TripoSR is selected or is the automatic recommendation.

Still2Solid falls back to Mock3D when TripoSR is absent, incomplete, unsupported or not selected. This is intentional.

## TripoSR installation cannot start

The current development milestone requires a discoverable **Python 3.11 or 3.12** in order to create the private runtime.

Check:

```bash
python3 --version
```

or, depending on the platform:

```bash
python --version
```

The final end-user release is intended to bundle this runtime in M7. Do not change the production worker to use an arbitrary global Python environment as a workaround.

## Installation fails verification

Do not bypass the checksum.

A verification failure means Still2Solid did not get the exact pinned asset it expected. Retry the install after confirming the network connection. If it continues failing, capture the error and compare the pin/checksum metadata in `docs/M3.md` and the Rust installer code.

The correct fix is to understand why the expected asset changed or could not be retrieved — not to disable verification.

## 8 GB Apple Silicon says “Memory constrained”

That is expected under the current policy.

The application can expose an explicit experimental install, but it does not automatically recommend TripoSR on 8 GB Apple Silicon. The real M1/8 GB benchmark is still an open release-validation item.

Start with:

- **Fast** quality;
- background removal only when useful;
- no other memory-heavy applications if you are deliberately benchmarking the constrained path;
- CPU if Metal/MPS produces an execution or memory error.

Do not interpret one successful run as proof that every input/quality setting is safe.

## Metal / MPS fails

Use **Advanced → Backend → CPU** and retry. Metal/MPS availability is not the same as full model compatibility.

If CPU works and MPS does not, record the machine, memory, macOS version, quality preset and failing stage. That is useful compatibility evidence.

## CUDA is not detected

Still2Solid detects NVIDIA acceleration through `nvidia-smi`.

Check that `nvidia-smi` is installed and works in the environment launching Still2Solid. If it cannot report the GPU, the app cannot safely infer the VRAM/backend from it.

## Generation looks stuck

Look at the stage name first. Some stages have long periods where the upstream operation cannot report fine-grained internal progress.

M4's ETA is learned from successful comparable runs on the same hardware/model/quality/backend/background-removal combination. On a first run, the app deliberately avoids inventing a confident countdown.

If cancellation works, cancel and retry once. If the same stage repeatedly hangs, capture the stage/error rather than repeatedly killing the whole application.

## ETA seems wrong

The learned profile is local and configuration-specific.

In **Advanced → Local timing profile** you can see the sample count, median, variation and recent accepted/excluded runs. Use **Reset** only if the profile is clearly no longer representative, for example after a major runtime/hardware change.

Failed and cancelled runs do not train the profile.

## Background removal was suggested incorrectly

The Background check is a heuristic, not a segmentation model. It looks at a tiny local sample of transparency and edge/centre colour statistics.

Simply toggle **Remove background** off. The same setting appears in Advanced as foreground isolation.

Typical cases where you may want it off:

- the object fills almost the whole frame;
- the “background” is physically part of what should become geometry;
- transparency/segmentation would remove thin desired parts;
- the source image is already carefully masked.

Typical cases where you normally want it on:

- object on a table/floor/shelf;
- room or outdoor scenery behind the object;
- plain studio/photo-paper background;
- obvious surrounding clutter.

## Background removal damages thin details

Turn it off and regenerate. Foreground isolation is optional and should never be treated as mandatory preprocessing.

If the original background is simple, a clean source crop may be better than aggressive segmentation for very thin structures.

## Production preview says the GLB is invalid

Still2Solid validates the GLB header and declared byte length before treating it as the canonical master.

An invalid production GLB should be treated as a generation/runtime defect. Do not “fix” the validator to accept malformed bytes. Keep the failing result/error and reproduce with the same input/settings.

## OBJ textures are missing

OBJ/MTL is a compatibility format. Still2Solid exports browser-readable base-colour and normal textures when possible, but it does not promise full PBR equivalence with GLB.

Use the GLB master when material fidelity matters.

## STL is the wrong physical size

There are two STL paths:

- **raw STL** from Export — geometry-only and intentionally has no reliable physical scale;
- **prepared STL** from Print Prep — coordinates are scaled to the millimetre dimensions you chose, but STL still does not contain a formal unit declaration.

For printing, prefer **3MF**, which explicitly declares millimetres.

## Print Prep says “Automatic repair incomplete”

This means unresolved topology remains after the conservative repair pass.

Still2Solid intentionally stops rather than inventing aggressive geometry. Inspect the model in a dedicated mesh-repair or slicer tool, or regenerate from a cleaner input.

Common causes include:

- large missing regions;
- intersecting geometry;
- complex non-manifold structures;
- multiple disconnected shells that are not intentional;
- holes that are not simple planar loops.

## The browser preview has no useful hardware information

Expected. Native hardware probing is a Tauri capability. Run:

```bash
npm run tauri:dev
```

Do not use the browser-only `npm run dev` view as hardware benchmark evidence.

## CI fails

Reproduce the same groups locally:

```bash
npm run check
npm run test
npm run build
python3 -m py_compile workers/triposr_worker.py
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Fix the failing layer without replacing unrelated working behavior.

## Still stuck?

When opening an issue, include the smallest useful diagnostic set:

- Still2Solid commit/version;
- OS and architecture;
- detected memory/accelerator;
- selected model and backend;
- quality preset;
- whether foreground isolation was on;
- failing stage;
- exact error text;
- whether Mock3D still works.

Do **not** upload a private source image unless it is necessary and you are comfortable making it available to the issue participants.
