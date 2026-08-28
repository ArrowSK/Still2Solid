Still2Solid bundled Python runtime staging directory.

This small source-tree placeholder exists so Tauri's configured resource glob
(`resources/python/**/*`) is valid during ordinary `cargo check` / `cargo test`
runs where the release runtime has not been prepared.

Release builds run `scripts/prepare_python_runtime.py` before Tauri packaging.
That script removes this directory and replaces it with the checksum-verified,
pinned Python runtime for the target platform, so this placeholder is not part
of the final packaged runtime.
