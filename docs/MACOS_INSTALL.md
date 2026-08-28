# Still2Solid on macOS

This is the normal, non-technical installation route for Apple Silicon Macs.

## Install

1. Download the `Still2Solid-macOS-Apple-Silicon` build artifact and open the `.dmg` inside it.
2. Drag **Still2Solid** into **Applications**.
3. Eject the Still2Solid disk image.
4. Open **Still2Solid** from Applications or Launchpad.

The application carries its own pinned Python runtime. The user does not need Terminal, Homebrew, Conda or a separate Python installation.

The first production model is installed from **Models** inside Still2Solid. Model weights are intentionally not embedded in the application package so the base installer stays manageable and model licences remain explicit.

### macOS security

When Apple Developer signing and notarization credentials are configured in the repository, the DMG follows the normal Gatekeeper path with no special user steps.

A private test build created without those credentials is still a normal DMG, but macOS may require the owner of the Mac to approve the application in Privacy & Security before first launch. Still2Solid must not instruct users to disable Gatekeeper or run quarantine-removal commands.

## Uninstall

1. If production models are installed and you want to reclaim their disk space, open **Still2Solid → Models** and use **Uninstall** for each installed model.
2. Quit Still2Solid.
3. Open **Applications** in Finder.
4. Move **Still2Solid** to the Trash.
5. Empty the Trash whenever convenient.

That is the standard macOS uninstall route. No terminal command or custom system-level uninstaller is required.

## Updates

A newer DMG can be installed by dragging the newer Still2Solid app into Applications and choosing **Replace** when macOS asks. Installed model data is kept separately, so an ordinary application update does not require downloading the models again.

## Target machine note

The application package itself supports Apple Silicon. TripoSR on an 8 GB Apple Silicon Mac remains a memory-constrained/experimental model path until the physical target-machine benchmark is completed; packaging success is not treated as proof of model performance.
