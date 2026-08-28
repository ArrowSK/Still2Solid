# Branding

Still2Solid's visual identity is built around the same product idea: a still image becoming a dimensional object.

The logo combines a flat source/object motif, a transformation path and a reconstructed wireframe form. The blue/cyan accent matches the application's dark technical UI without turning the workspace into a decorative 3D demo.

## Canonical assets

| Asset | Purpose |
| --- | --- |
| `assets/branding/still2solid-logo.svg` | Canonical full logo/wordmark for README and project-facing material. |
| `assets/branding/still2solid-icon.svg` | Canonical square icon master. |
| `apps/desktop/ui/public/brand/still2solid-icon.svg` | In-app copy used by the top bar and drop zone. |
| `src-tauri/icons/*` | Generated native platform derivatives, including PNG, ICNS and ICO assets. |

The README uses the SVG wordmark directly so GitHub renders the logo sharply and without the earlier broken/odd WebP presentation. The application uses the matching SVG square mark, and native Tauri icons are derivatives of the same master rather than an independent design.

The previous 1×1 technical placeholder is not used as product branding.

## UI usage

The in-app icon is intentionally small and quiet:

- top-left application identity;
- opening image drop zone;
- no repeated watermarking in the workspace;
- no animated logo effects;
- no oversized hero artwork that reduces working space.

The product name remains real text in the application for accessibility and legibility rather than being replaced by an image wordmark.

## Colour direction

The identity uses near-black/deep navy surfaces, cool grey/white object and text tones, a blue-to-cyan transformation accent and blue wireframe detail. The application should continue to use the mark as an accent rather than recolouring every control around it.

## Native icon rule

OS/application icons use the square artwork, never the full `Still2Solid` wordmark. Small icon sizes must preserve the source/reconstruction relationship and enough breathing room to remain legible.

When the square master changes, regenerate all platform derivatives together. Do not hand-edit `icon.icns`, `icon.ico` or individual PNG sizes into independent variants.

## Accessibility

When the full logo is embedded in HTML/Markdown, use an alt label such as:

> Still2Solid — local image to 3D

In the desktop UI the small icon is decorative because the adjacent `Still2Solid` heading provides the accessible name.

## Packaging

M7 generated the native platform icon set from the checked-in square master and wired those assets into active Tauri bundling. Release validation should still inspect the actual packaged macOS/Windows/Linux artifacts because generated-file existence alone cannot prove OS presentation.

## Ownership/licensing note

The Still2Solid brand artwork is project-original artwork created for this application and contains no third-party company/model logos. Repository code is Apache-2.0; branding usage may additionally be affected by normal trademark/name rights if the project later establishes them. Do not assume a model provider's logo or branding is licensed merely because that model is supported by Still2Solid.
