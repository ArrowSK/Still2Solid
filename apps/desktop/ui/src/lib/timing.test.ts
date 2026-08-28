import { describe, expect, it } from 'vitest';
import {
  deriveStageSeconds,
  estimateProgressFromTiming,
  median,
  shouldAcceptTimingSample,
  summarizeTimingProfile,
} from './timing';
import type { ProgressEvent } from './types';

function progress(stageId: string, elapsedSeconds: number, stageProgress: number): ProgressEvent {
  return {
    stageId,
    stageName: stageId,
    stageProgress,
    overallProgress: 0,
    progressIsEstimated: true,
    elapsedSeconds,
    etaSeconds: 0,
    etaConfidence: 'low',
    statusMessage: stageId,
  };
}

describe('timing profiles', () => {
  it('computes a stable median', () => {
    expect(median([12, 8, 10])).toBe(10);
    expect(median([8, 10, 12, 14])).toBe(11);
  });

  it('derives durations from sparse M3 worker progress', () => {
    const stages = deriveStageSeconds([
      progress('prepare', 1, 0.1),
      progress('isolate', 5, 1),
      progress('load', 5.1, 0.1),
      progress('load', 9, 1),
      progress('reconstruct', 9.1, 0.05),
      progress('reconstruct', 19, 1),
      progress('preview', 19.2, 0.5),
      progress('preview', 20, 1),
    ], 20);

    expect(stages.prepare).toBeCloseTo(1);
    expect(stages.isolate).toBeCloseTo(4);
    expect(stages.load).toBeCloseTo(3.9);
    expect(stages.reconstruct).toBeCloseTo(9.9);
    expect(stages.preview).toBeGreaterThan(0);
  });

  it('excludes extreme successful outliers after a baseline exists', () => {
    const baseline = [30, 31, 29, 32, 30.5];
    expect(shouldAcceptTimingSample(baseline, 34)).toBe(true);
    expect(shouldAcceptTimingSample(baseline, 120)).toBe(false);
    expect(shouldAcceptTimingSample([], 120)).toBe(true);
  });

  it('uses learned stage weights to produce ETA and progress', () => {
    const summary = summarizeTimingProfile('key', {
      context: {
        hardwareKey: 'hardware',
        modelId: 'triposr',
        modelVersion: 'v1',
        quality: 'standard',
        backend: 'auto',
        backgroundRemoval: true,
      },
      updatedAt: 1,
      samples: [
        {
          completedAt: 1,
          totalSeconds: 30,
          resolvedBackend: 'mps',
          accepted: true,
          stageSeconds: { prepare: 3, reconstruct: 18, preview: 9 },
        },
      ],
    });

    const estimated = estimateProgressFromTiming(
      progress('reconstruct', 7, 0.25),
      summary,
      ['prepare', 'reconstruct', 'preview'],
      0,
    );

    expect(estimated.overallProgress).toBeCloseTo(0.25);
    expect(estimated.etaSeconds).toBeCloseTo(22.5);
    expect(estimated.etaConfidence).toBe('low');
  });
});
