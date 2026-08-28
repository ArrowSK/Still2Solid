# Background Guidance

Still2Solid can optionally isolate the foreground object before TripoSR inference. The setting already existed in Advanced mode; the product-facing improvement in 0.6.1 is that the app now evaluates the selected image and explains whether isolation is likely to help.

## Design goal

A user should not need to know what “foreground isolation” means before getting a clean first result. At the same time, Still2Solid should not silently rewrite every image.

The intended experience is:

1. choose an image;
2. Still2Solid runs a tiny local background check;
3. the UI says whether a background is likely present, already transparent, or uncertain;
4. **Remove background** is suggested accordingly;
5. the user can override the choice;
6. the final choice is passed into the existing production-generation option.

## What the check actually does

The check is deliberately lightweight and deterministic. It downsamples the image locally and samples:

- transparency near the image boundary;
- colour spread around the boundary;
- colour distance between boundary and centre.

This is not semantic object recognition and it does not try to identify the subject.

### Likely background

Opaque edge pixels are relatively coherent or measurably different from the centre. This pattern is common for an object photographed against a wall, paper, table, floor or surrounding scene.

### Already isolated

A meaningful amount of transparency reaches the image boundary. This usually means the subject has already been cut out.

### Uncertain

The image is opaque but the edge statistics are too busy to make a strong call. Still2Solid keeps the option available and explains the uncertainty instead of pretending the heuristic knows more than it does.

## Privacy

The analysis is performed in the desktop webview on a downscaled local canvas. It does not upload the image, persist the sample or invoke an external vision service.

## Actual removal

When **Remove background** is enabled for a production TripoSR job, Still2Solid uses the existing verified local foreground-isolation runtime asset from M3. The background check itself does not remove pixels; it only recommends the option.

## Failure behaviour

If the background check cannot decode/analyse the image, generation should remain available. The UI falls back to a manual foreground-isolation choice rather than treating advisory analysis as a hard prerequisite.

## Testing

The pure pixel-analysis function has unit tests covering:

- transparent edges;
- simple opaque background around a central subject;
- ambiguous/busy opaque imagery;
- invalid pixel buffers.

The heuristic should remain conservative and cheap. A future replacement with a learned classifier would require a separate runtime, licence, privacy and performance review rather than being slipped into this helper unnoticed.
