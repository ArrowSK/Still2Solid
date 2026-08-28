import { describe, expect, it } from 'vitest';
import { assessBackgroundPixels } from './backgroundAnalysis';

function image(width: number, height: number, pixel: (x: number, y: number) => [number, number, number, number]) {
  const data = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const [r, g, b, a] = pixel(x, y);
      const offset = (y * width + x) * 4;
      data[offset] = r;
      data[offset + 1] = g;
      data[offset + 2] = b;
      data[offset + 3] = a;
    }
  }
  return data;
}

describe('background assessment', () => {
  it('recognizes transparent image edges as already isolated', () => {
    const width = 32;
    const height = 32;
    const data = image(width, height, (x, y) => {
      const edge = x < 5 || y < 5 || x >= width - 5 || y >= height - 5;
      return edge ? [0, 0, 0, 0] : [180, 120, 80, 255];
    });

    const result = assessBackgroundPixels(data, width, height);
    expect(result.kind).toBe('transparent');
    expect(result.suggestRemoval).toBe(false);
  });

  it('recommends removal for a subject on a uniform opaque background', () => {
    const width = 40;
    const height = 40;
    const data = image(width, height, (x, y) => {
      const subject = x > 12 && x < 28 && y > 9 && y < 32;
      return subject ? [80, 80, 90, 255] : [244, 244, 240, 255];
    });

    const result = assessBackgroundPixels(data, width, height);
    expect(result.kind).toBe('likely-background');
    expect(result.suggestRemoval).toBe(true);
    expect(['high', 'medium']).toContain(result.confidence);
  });

  it('keeps the removal option enabled when an opaque scene is ambiguous', () => {
    const width = 36;
    const height = 36;
    const data = image(width, height, (x, y) => {
      const value = (x * 47 + y * 31) % 255;
      return [value, (value * 3) % 255, (value * 7) % 255, 255];
    });

    const result = assessBackgroundPixels(data, width, height);
    expect(result.suggestRemoval).toBe(true);
    expect(['likely-background', 'uncertain']).toContain(result.kind);
  });

  it('rejects invalid pixel buffers', () => {
    expect(() => assessBackgroundPixels(new Uint8ClampedArray(4), 10, 10)).toThrow(/invalid pixel data/i);
  });
});
