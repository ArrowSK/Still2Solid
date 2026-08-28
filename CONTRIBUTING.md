# Contributing to Still2Solid

Thanks for helping improve Still2Solid. The project values small, reviewable changes and clear user-facing behavior over cleverness.

## Before changing code

Read:

- [Development](docs/DEVELOPMENT.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Security & Privacy](docs/SECURITY_PRIVACY.md)
- [Model Licence Policy](docs/MODEL_LICENSE_POLICY.md) if the change touches models/runtimes

## Ground rules

1. Preserve working behavior unless the change intentionally modifies it.
2. Fix the layer that is broken; do not redesign unrelated layers as a shortcut.
3. Keep local-first behavior intact.
4. Do not add telemetry, persistent localhost inference servers or arbitrary remote model code.
5. Do not bypass model/source verification to make installation “work.”
6. Keep the canonical GLB non-destructive.
7. Prefer an explicit “repair incomplete / unsupported / uncertain” state over pretending a result is safe.
8. Write user-facing text for people who do not know the internal architecture.

## Development checks

Before requesting merge, run:

```bash
npm install
npm run check
npm run test
npm run build
python3 -m py_compile workers/triposr_worker.py
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

CI should remain independent of a GPU and should not download full production model weights.

## Pull requests

A good PR has:

- one coherent purpose;
- a clear explanation of user-visible behavior;
- tests for deterministic new logic;
- documentation changes when behavior/policy changes;
- no unexplained licence/security expansion;
- a clean CI run.

Large refactors should explain why a smaller change cannot solve the problem.

## Model changes

Do not add a model just because it performs well in a demo. Include:

- exact upstream project/model identity;
- licence and regional restrictions;
- source/model revision pins;
- verification method;
- runtime/dependency implications;
- hardware floor;
- backend support actually tested;
- whether any remote code is required;
- output/export compatibility;
- cleanup/cancellation behavior.

Conditional/gated models must be clearly labelled and must not silently become automatic choices.

## Documentation

Documentation is part of the feature. Keep the README and relevant guide aligned with the actual implementation. Distinguish clearly between:

- implemented;
- experimentally available;
- validated/recommended;
- planned.

## Security issues

Do not publish actionable vulnerability details in a normal issue. Follow [SECURITY.md](SECURITY.md).
