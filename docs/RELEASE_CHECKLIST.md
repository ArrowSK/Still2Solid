# Release Checklist

This checklist defines the minimum bar for the planned M7 end-user release. It is intentionally stricter than “the app builds on a developer machine.”

## Product

- [ ] Simple image → background guidance → quality → generate → preview flow works without opening Advanced.
- [ ] Model Manager explains the selected model and hardware caveats.
- [ ] Mock3D fallback remains usable if production runtime is unavailable.
- [ ] Cancellation leaves the application usable for the next run.
- [ ] GLB, OBJ package and raw STL exports work.
- [ ] Print Prep can produce 3MF and prepared STL.
- [ ] `Printable` and `Automatic repair incomplete` wording remains honest.

## Target hardware

- [ ] Primary Apple Silicon target has been tested from a clean install.
- [ ] 8 GB M1 recommendation status reflects measured results, not assumptions.
- [ ] At least one additional supported platform/backend has a clean-install smoke test.
- [ ] Memory-constrained configurations fail gracefully.

## Runtime packaging

- [ ] Normal users do not need to install Python manually.
- [ ] Runtime/model storage is versioned and separate from application assets.
- [ ] Pinned source/model verification remains enabled.
- [ ] Failed/stopped install cannot activate an incomplete runtime.
- [ ] Reinstall/uninstall leaves predictable state.
- [ ] Offline generation works after successful installation.

## Platform packaging

- [ ] Tauri bundle is enabled for release builds.
- [ ] Native icon derivatives come from the checked-in Still2Solid square master.
- [ ] macOS package is signed and notarized.
- [ ] Windows signing/reputation strategy is documented and applied where applicable.
- [ ] Release artifacts have stable names and checksums.

## Privacy/security

- [ ] No telemetry or analytics introduced.
- [ ] No cloud inference fallback introduced silently.
- [ ] No persistent localhost inference server.
- [ ] CSP changes reviewed.
- [ ] No production `trust_remote_code=True`.
- [ ] Background check remains local.
- [ ] Timing history contains no image/filename content.

## Quality gates

- [ ] `npm run check`
- [ ] `npm run test`
- [ ] `npm run build`
- [ ] worker syntax validation
- [ ] `cargo check`
- [ ] `cargo test`
- [ ] clean-install smoke test
- [ ] first real generation smoke test
- [ ] export smoke test
- [ ] print-prep smoke test

## Documentation/licensing

- [ ] README version/status is current.
- [ ] Getting Started describes the actual release experience rather than development-only prerequisites.
- [ ] User Guide screenshots/text match the release UI.
- [ ] Roadmap distinguishes shipped from planned.
- [ ] Third-party notices reviewed.
- [ ] Model licence review re-run for every bundled/supported production model.
- [ ] SECURITY.md reporting route confirmed.

## Release notes

Release notes should answer, in plain language:

- what Still2Solid can do;
- supported/recommended hardware;
- known limitations;
- what the user must download on first run;
- where models/runtime are stored;
- privacy behavior;
- major changes since the prior release.

Do not advertise the 15–30 second generation target as a universal fact unless measurements support it for the named hardware/model/quality combination.
