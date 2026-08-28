# GitHub releases

Still2Solid publishes user-installable builds through GitHub Releases.

## macOS Apple Silicon family preview

The `macOS Family Package` workflow builds a normal `.dmg` for Apple Silicon Macs, bundles the pinned Python runtime, creates a SHA-256 checksum, and can publish the result as a GitHub prerelease.

A release branch named `release/...` triggers publication. The published package remains a prerelease until the physical M1 8 GB validation and Apple Developer signing/notarization gates are complete.

### Install

1. Download the `.dmg` from the GitHub Release.
2. Open it.
3. Drag **Still2Solid** to **Applications**.
4. Open Still2Solid from Applications or Launchpad.

No Terminal, Homebrew, Conda, or separate Python installation is required.

### Update

Download the newer DMG and drag Still2Solid to Applications again. Choose **Replace** when Finder asks. Installed model data is kept separately from the application bundle.

### Uninstall

1. Optionally open **Models** in Still2Solid and uninstall downloaded models to reclaim their disk space.
2. Quit Still2Solid.
3. Open Finder → Applications.
4. Move Still2Solid to Trash.

This follows the normal macOS application removal path.

### Signing status

If Apple Developer signing credentials are configured in GitHub Actions, the workflow builds with those credentials. Without them, the family-preview build is intentionally produced unsigned and the GitHub release notes say so clearly. An unsigned build may require **Open Anyway** in macOS Privacy & Security on first launch.
