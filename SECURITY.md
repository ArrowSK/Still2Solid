# Security Policy

## Supported code

Still2Solid is currently a pre-release project. Security fixes are made against the current `main` branch; old milestone snapshots are historical documentation rather than separately supported release lines.

## Reporting a vulnerability

Please avoid posting an exploitable vulnerability, malicious payload or sensitive reproduction details in a normal public GitHub issue while the issue is still actionable.

Prefer GitHub's **private vulnerability reporting / Security Advisory** flow for this repository when it is available. If that flow is not available, contact the repository owner through GitHub and ask for a private channel before sending exploit details.

A useful report includes:

- affected commit/version;
- operating system and architecture;
- concise impact statement;
- reproduction steps;
- whether the issue requires a model/runtime to be installed;
- whether network access is required;
- the smallest safe proof of concept;
- suggested mitigation if known.

Do not include private source images or unrelated personal data.

## Security design notes

Still2Solid intentionally avoids several common attack surfaces:

- no cloud inference in the normal generation flow;
- no telemetry/analytics;
- no persistent localhost inference HTTP service;
- no `trust_remote_code=True` production model loading;
- allowlisted, pinned and verified production model/runtime downloads;
- one-shot model worker lifecycle;
- explicit Tauri Content Security Policy;
- local-only background assessment, timing and export/print processing.

See [docs/SECURITY_PRIVACY.md](docs/SECURITY_PRIVACY.md) for the complete architecture boundary.
