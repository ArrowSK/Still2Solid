# Still2Solid on macOS

This is the normal, non-technical installation route for Apple Silicon Macs.

## Install

1. Download the current Still2Solid Apple Silicon `.dmg` from GitHub Releases.
2. Open the DMG and drag **Still2Solid** into **Applications**.
3. Eject the Still2Solid disk image.
4. Open **Still2Solid** from Applications or Launchpad.

The application carries its own pinned Python runtime. The user does not need Terminal, Homebrew, Conda or a separate Python installation.

Production models are installed from **Models** inside Still2Solid. Model weights are intentionally not embedded in the application package so the base installer stays manageable and model licences remain explicit.

### macOS security

When Apple Developer signing and notarization credentials are configured in the repository, the DMG follows the normal Gatekeeper path with no special user steps.

A build created without those credentials is still a normal DMG, but macOS may require the owner of the Mac to approve the application in **System Settings → Privacy & Security → Open Anyway** before first launch. Still2Solid must not instruct users to disable Gatekeeper or run quarantine-removal commands.

## Uninstall

1. If production models are installed and you want to reclaim their disk space, open **Still2Solid → Settings** and remove downloaded models, or use **Uninstall** for individual models in **Models**.
2. If you want a complete cleanup, use **Settings → Prepare for uninstall** first.
3. Quit Still2Solid.
4. Open **Applications** in Finder.
5. Move **Still2Solid** to the Trash.
6. Empty the Trash whenever convenient.

That is the standard macOS uninstall route. No terminal command or custom system-level uninstaller is required.

## Updates from inside the app

Still2Solid 0.8.3 and later include a manual updater in **Settings → Software Update**.

1. Press **Check for updates**. No update request is made until you explicitly press this button.
2. If a newer GitHub Release is available, press **Download & open update**.
3. Still2Solid downloads the Apple Silicon DMG into its own cache and verifies the file against the release SHA-256 digest/checksum.
4. After verification, Still2Solid opens the normal macOS DMG.
5. Drag **Still2Solid** into **Applications** and choose **Replace**.

Downloaded model data lives separately from the application bundle, so an ordinary application update keeps already-installed models. Downloaded update installers are app cache and can be reclaimed with **Settings → Clear temporary files**.

The updater intentionally preserves the normal macOS replacement route rather than silently replacing the running application. This keeps the process understandable and avoids bypassing Gatekeeper or filesystem permission checks.

## Target machine note

The application package itself supports Apple Silicon. TripoSR on an 8 GB Apple Silicon Mac remains a memory-constrained/experimental model path until the physical target-machine benchmark is completed; packaging success is not treated as proof of model performance.
