import { describe, expect, it } from 'vitest';
import { formatAssetBytes, safeAssetBaseName, validateGlbHeader } from './assetExport';

function glbHeader(length = 12, version = 2, magic = 0x46546c67): Uint8Array {
  const bytes = new Uint8Array(length);
  const view = new DataView(bytes.buffer);
  view.setUint32(0, magic, true);
  view.setUint32(4, version, true);
  view.setUint32(8, length, true);
  return bytes;
}

describe('M5 canonical asset helpers', () => {
  it('sanitizes export base names and removes model extensions', () => {
    expect(safeAssetBaseName('My chair 01.glb')).toBe('My-chair-01');
    expect(safeAssetBaseName('  ///.stl')).toBe('still2solid-model');
  });

  it('accepts an exact GLB 2.0 header', () => {
    expect(validateGlbHeader(glbHeader())).toEqual({ version: 2, length: 12 });
  });

  it('rejects wrong magic, version and declared length', () => {
    expect(() => validateGlbHeader(glbHeader(12, 2, 0))).toThrow(/not a GLB/i);
    expect(() => validateGlbHeader(glbHeader(12, 1))).toThrow(/version 1/i);
    const bytes = glbHeader(16);
    new DataView(bytes.buffer).setUint32(8, 12, true);
    expect(() => validateGlbHeader(bytes)).toThrow(/length mismatch/i);
  });

  it('formats asset sizes for UI diagnostics', () => {
    expect(formatAssetBytes(512)).toBe('512 B');
    expect(formatAssetBytes(2048)).toBe('2.0 KB');
    expect(formatAssetBytes(2 * 1024 * 1024)).toBe('2.0 MB');
  });
});
