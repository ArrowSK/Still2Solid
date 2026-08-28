# Branding

Still2Solid's visual identity is built around the same idea as the product: a still image becoming a dimensional object.

The logo shows a photographed object on the left, a transformation path through the centre and a reconstructed/wireframe object on the right. The blue/cyan accent matches the application's existing dark technical UI without turning the interface into a decorative 3D demo.

## Repository assets

| Asset | Purpose |
| --- | --- |
| `assets/branding/still2solid-logo.webp` | Full logo/wordmark for README and project-facing material. |
| `apps/desktop/ui/public/brand/still2solid-icon.webp` | Lightweight in-app brand icon used in the top bar and drop zone. |
| `src-tauri/icons/icon.png` | RGBA desktop application icon source for Tauri/development builds. |

The previous 1×1 technical placeholder icon is no longer the product icon.

## UI usage

The in-app icon is intentionally small and quiet:

- top-left application identity;
- opening drop zone;
- no repeated watermarking in the workspace;
- no animated logo effects;
- no large decorative hero that reduces working space.

The product name remains rendered as text in the application for accessibility and legibility rather than being replaced by an image wordmark.

## Colour direction

The logo uses the existing product palette:

- near-black / deep navy background;
- cool grey/white text and ceramic tones;
- blue → cyan transformation accent;
- blue wireframe/grid detail.

The app should continue to use the logo as an accent rather than recolouring every control to match it.

## Icon crop

Desktop/app icons use the square icon artwork rather than the full wordmark. The crop should preserve:

- source image/card on the left;
- reconstructed object on the right;
- transformation arrow;
- enough breathing room around the rounded-square motif.

Do not use the full `Still2Solid` text inside small OS icons; it becomes unreadable and visually noisy.

## Accessibility

When the full logo is embedded in HTML/Markdown, use an alt label such as:

> Still2Solid — local image to 3D

In the desktop UI the icon is decorative because the nearby `Still2Solid` heading already provides the accessible name.

## Packaging

M7 should generate/check the complete native icon set required by the release targets (for example macOS ICNS and Windows ICO derivatives) from the checked-in square master artwork as part of the release process. Generated platform derivatives should not become independent brand sources.

## Ownership/licensing note

The Still2Solid brand artwork is project-original artwork created for this application and contains no third-party company/model logos. Repository code is Apache-2.0; branding usage may additionally be affected by normal trademark/name rights if the project later establishes them. Do not assume a model provider's logo or branding is licensed merely because that model is supported by Still2Solid.
