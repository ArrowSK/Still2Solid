export type BackgroundAssessmentKind = 'likely-background' | 'transparent' | 'uncertain';
export type BackgroundConfidence = 'high' | 'medium' | 'low';

export interface BackgroundAssessment {
  kind: BackgroundAssessmentKind;
  confidence: BackgroundConfidence;
  suggestRemoval: boolean;
  reason: string;
  edgeTransparency: number;
  edgeColorSpread: number;
  edgeCenterDistance: number;
}

interface PixelStats {
  count: number;
  r: number;
  g: number;
  b: number;
  r2: number;
  g2: number;
  b2: number;
}

function emptyStats(): PixelStats {
  return { count: 0, r: 0, g: 0, b: 0, r2: 0, g2: 0, b2: 0 };
}

function addPixel(stats: PixelStats, r: number, g: number, b: number) {
  stats.count += 1;
  stats.r += r;
  stats.g += g;
  stats.b += b;
  stats.r2 += r * r;
  stats.g2 += g * g;
  stats.b2 += b * b;
}

function mean(stats: PixelStats): [number, number, number] {
  if (!stats.count) return [0, 0, 0];
  return [stats.r / stats.count, stats.g / stats.count, stats.b / stats.count];
}

function colorSpread(stats: PixelStats): number {
  if (!stats.count) return 0;
  const [r, g, b] = mean(stats);
  const vr = Math.max(0, stats.r2 / stats.count - r * r);
  const vg = Math.max(0, stats.g2 / stats.count - g * g);
  const vb = Math.max(0, stats.b2 / stats.count - b * b);
  return Math.sqrt((vr + vg + vb) / 3);
}

function colorDistance(a: [number, number, number], b: [number, number, number]): number {
  const dr = a[0] - b[0];
  const dg = a[1] - b[1];
  const db = a[2] - b[2];
  return Math.sqrt((dr * dr + dg * dg + db * db) / 3);
}

/**
 * Lightweight, deliberately conservative background heuristic.
 *
 * Still2Solid does not run a second AI model just to decide whether to offer
 * foreground isolation. Instead it examines a tiny local pixel sample:
 * transparent edges strongly suggest an already-isolated object; opaque edges
 * are treated as possible background, with confidence raised when the border is
 * visually coherent or distinct from the centre of the image.
 */
export function assessBackgroundPixels(
  data: Uint8ClampedArray,
  width: number,
  height: number,
): BackgroundAssessment {
  if (width <= 0 || height <= 0 || data.length < width * height * 4) {
    throw new Error('Background analysis received invalid pixel data.');
  }

  const border = Math.max(1, Math.round(Math.min(width, height) * 0.1));
  const centerX0 = Math.floor(width * 0.28);
  const centerX1 = Math.ceil(width * 0.72);
  const centerY0 = Math.floor(height * 0.28);
  const centerY1 = Math.ceil(height * 0.72);
  const step = Math.max(1, Math.floor(Math.min(width, height) / 64));

  const edgeStats = emptyStats();
  const centerStats = emptyStats();
  let edgeSamples = 0;
  let transparentEdgeSamples = 0;
  let allSamples = 0;
  let transparentSamples = 0;

  for (let y = 0; y < height; y += step) {
    for (let x = 0; x < width; x += step) {
      const offset = (y * width + x) * 4;
      const r = data[offset];
      const g = data[offset + 1];
      const b = data[offset + 2];
      const a = data[offset + 3];
      const transparent = a < 224;
      const isEdge = x < border || y < border || x >= width - border || y >= height - border;
      const isCenter = x >= centerX0 && x < centerX1 && y >= centerY0 && y < centerY1;

      allSamples += 1;
      if (transparent) transparentSamples += 1;

      if (isEdge) {
        edgeSamples += 1;
        if (transparent) transparentEdgeSamples += 1;
        else addPixel(edgeStats, r, g, b);
      }
      if (isCenter && a >= 224) addPixel(centerStats, r, g, b);
    }
  }

  const edgeTransparency = edgeSamples ? transparentEdgeSamples / edgeSamples : 0;
  const overallTransparency = allSamples ? transparentSamples / allSamples : 0;
  const edgeColorSpread = colorSpread(edgeStats);
  const edgeCenterDistance = edgeStats.count && centerStats.count
    ? colorDistance(mean(edgeStats), mean(centerStats))
    : 0;

  if (edgeTransparency >= 0.18 || overallTransparency >= 0.1) {
    return {
      kind: 'transparent',
      confidence: edgeTransparency >= 0.45 ? 'high' : 'medium',
      suggestRemoval: false,
      reason: 'Transparent pixels reach the image boundary, so the object already appears to be isolated.',
      edgeTransparency,
      edgeColorSpread,
      edgeCenterDistance,
    };
  }

  if (edgeColorSpread <= 42 && edgeCenterDistance >= 18) {
    return {
      kind: 'likely-background',
      confidence: 'high',
      suggestRemoval: true,
      reason: 'The outer image area is visually consistent and differs from the centre, which is typical of a subject on a background.',
      edgeTransparency,
      edgeColorSpread,
      edgeCenterDistance,
    };
  }

  if (edgeColorSpread <= 72 || edgeCenterDistance >= 26) {
    return {
      kind: 'likely-background',
      confidence: 'medium',
      suggestRemoval: true,
      reason: 'The local edge sample looks like a surrounding scene or surface rather than transparency.',
      edgeTransparency,
      edgeColorSpread,
      edgeCenterDistance,
    };
  }

  return {
    kind: 'uncertain',
    confidence: 'low',
    suggestRemoval: true,
    reason: 'The image is opaque and the quick local check cannot confidently separate subject from surroundings.',
    edgeTransparency,
    edgeColorSpread,
    edgeCenterDistance,
  };
}

export async function analyseImageBackground(imageUrl: string): Promise<BackgroundAssessment> {
  const image = new Image();
  image.decoding = 'async';
  image.src = imageUrl;
  await image.decode();

  const maxSide = 96;
  const scale = Math.min(1, maxSide / Math.max(image.naturalWidth, image.naturalHeight));
  const width = Math.max(1, Math.round(image.naturalWidth * scale));
  const height = Math.max(1, Math.round(image.naturalHeight * scale));
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d', { alpha: true, willReadFrequently: true });
  if (!context) throw new Error('Could not create a local image-analysis canvas.');
  context.drawImage(image, 0, 0, width, height);
  return assessBackgroundPixels(context.getImageData(0, 0, width, height).data, width, height);
}
