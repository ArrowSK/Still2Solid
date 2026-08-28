import { describe, expect, it } from 'vitest';
import { assessModels, recommendedProductionModel } from './recommendation';
import type { HardwareProfile } from './types';

function hardware(overrides: Partial<HardwareProfile> = {}): HardwareProfile {
  return {
    platform: 'macos',
    architecture: 'aarch64',
    chip: 'Apple Silicon',
    memoryGb: 16,
    osVersion: 'macOS',
    preferredBackend: 'Metal / MPS',
    accelerators: [
      { type: 'apple-unified', name: 'Apple Silicon GPU', memoryGb: 16, backend: 'metal' },
    ],
    supportsMetal: true,
    supportsCuda: false,
    ...overrides,
  };
}

describe('hardware-aware model recommendation', () => {
  it('recommends TripoSR on a 16 GB Apple Silicon Mac', () => {
    const assessments = assessModels(hardware());
    expect(recommendedProductionModel(assessments)?.modelId).toBe('triposr');
    expect(assessments.find((item) => item.modelId === 'sf3d')?.compatibility).toBe('memory-constrained');
    expect(assessments.find((item) => item.modelId === 'trellis2-4b')?.compatibility).toBe('unsupported');
  });

  it('does not claim a safe production recommendation on an 8 GB Apple Silicon Mac', () => {
    const assessments = assessModels(hardware({
      memoryGb: 8,
      accelerators: [{ type: 'apple-unified', name: 'Apple Silicon GPU', memoryGb: 8, backend: 'metal' }],
    }));
    expect(recommendedProductionModel(assessments)).toBeUndefined();
    expect(assessments.find((item) => item.modelId === 'triposr')?.compatibility).toBe('memory-constrained');
  });

  it('keeps TripoSR as the automatic permissive recommendation on a large Apple Silicon Mac', () => {
    const assessments = assessModels(hardware({
      memoryGb: 64,
      accelerators: [{ type: 'apple-unified', name: 'Apple Silicon GPU', memoryGb: 64, backend: 'metal' }],
    }));
    expect(recommendedProductionModel(assessments)?.modelId).toBe('triposr');
    expect(assessments.find((item) => item.modelId === 'sf3d')?.compatibility).toBe('compatible');
  });

  it('recommends TRELLIS.2 when Linux has the published 24 GB NVIDIA minimum', () => {
    const assessments = assessModels(hardware({
      platform: 'linux',
      architecture: 'x86_64',
      memoryGb: 64,
      preferredBackend: 'CUDA',
      accelerators: [{ type: 'nvidia', name: 'NVIDIA GPU', memoryGb: 24, backend: 'cuda' }],
      supportsMetal: false,
      supportsCuda: true,
    }));
    expect(recommendedProductionModel(assessments)?.modelId).toBe('trellis2-4b');
    expect(assessments.find((item) => item.modelId === 'triposr')?.compatibility).toBe('compatible');
  });

  it('does not manufacture a recommendation when native hardware probing is unavailable', () => {
    const assessments = assessModels(hardware({
      platform: 'Browser preview',
      architecture: 'unknown',
      memoryGb: 0,
      accelerators: [],
      supportsMetal: false,
      supportsCuda: false,
    }));
    expect(recommendedProductionModel(assessments)).toBeUndefined();
  });
});
