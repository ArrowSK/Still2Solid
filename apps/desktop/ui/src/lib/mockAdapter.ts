import type {
  GenerationRequest,
  GenerationResult,
  ModelAdapter,
  ModelManifest,
  ProgressEvent,
  QualityPreset,
} from './types';

const STAGES: ModelManifest['stages'] = [
  { id: 'prepare', label: 'Preparing image' },
  { id: 'isolate', label: 'Isolating object' },
  { id: 'load', label: 'Loading model' },
  { id: 'reconstruct', label: 'Reconstructing geometry' },
  { id: 'mesh', label: 'Extracting mesh' },
  { id: 'texture', label: 'Baking texture' },
  { id: 'preview', label: 'Preparing preview' },
];

const DURATIONS_MS: Record<QualityPreset, number[]> = {
  fast: [180, 220, 220, 700, 420, 500, 180],
  standard: [220, 280, 260, 1100, 650, 850, 220],
  best: [260, 340, 300, 1650, 950, 1250, 250],
};

const TRIANGLES: Record<QualityPreset, number> = {
  fast: 24_000,
  standard: 72_000,
  best: 148_000,
};

export const mockManifest: ModelManifest = {
  id: 'mock3d',
  name: 'Mock3D',
  family: 'still2solid-development',
  version: '1.0.0',
  license: 'No external model weights',
  licenseStatus: 'not-applicable',
  supportsTexture: true,
  supportsUv: true,
  supportsPbr: false,
  diskSizeMb: 0,
  stages: STAGES,
};

function sleep(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(resolve, ms);
    signal.addEventListener(
      'abort',
      () => {
        window.clearTimeout(timer);
        reject(new DOMException('Generation cancelled', 'AbortError'));
      },
      { once: true },
    );
  });
}

export class Mock3DAdapter implements ModelAdapter {
  manifest = mockManifest;

  async generate(
    request: GenerationRequest,
    onProgress: (event: ProgressEvent) => void,
    signal: AbortSignal,
  ): Promise<GenerationResult> {
    const durations = DURATIONS_MS[request.quality];
    const totalMs = durations.reduce((sum, value) => sum + value, 0);
    const started = performance.now();
    let completedMs = 0;

    for (let stageIndex = 0; stageIndex < STAGES.length; stageIndex += 1) {
      const stage = STAGES[stageIndex];
      const stageDuration = durations[stageIndex];
      const tickMs = 50;
      let stageElapsed = 0;

      while (stageElapsed < stageDuration) {
        if (signal.aborted) throw new DOMException('Generation cancelled', 'AbortError');
        const slice = Math.min(tickMs, stageDuration - stageElapsed);
        await sleep(slice, signal);
        stageElapsed += slice;

        const elapsedMs = completedMs + stageElapsed;
        const event: ProgressEvent = {
          stageId: stage.id,
          stageName: stage.label,
          stageProgress: Math.min(1, stageElapsed / stageDuration),
          overallProgress: Math.min(1, elapsedMs / totalMs),
          progressIsEstimated: true,
          elapsedSeconds: (performance.now() - started) / 1000,
          etaSeconds: Math.max(0, (totalMs - elapsedMs) / 1000),
          etaConfidence: 'high',
          statusMessage: `${stage.label} · deterministic development adapter`,
        };
        onProgress(event);
      }

      completedMs += stageDuration;
    }

    return {
      jobId: crypto.randomUUID(),
      modelId: this.manifest.id,
      quality: request.quality,
      elapsedSeconds: (performance.now() - started) / 1000,
      triangles: TRIANGLES[request.quality],
      textured: true,
      metadata: {
        backend: request.backend,
        backgroundRemoval: request.backgroundRemoval,
        sourceName: request.sourceName,
        sourceSizeBytes: request.sourceSizeBytes,
        adapter: 'Mock3D',
      },
    };
  }
}
