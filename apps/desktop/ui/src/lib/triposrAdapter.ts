import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { modelCandidateById } from './modelCatalog';
import type {
  GenerationRequest,
  GenerationResult,
  ModelAdapter,
  ProgressEvent,
} from './types';

interface TripoResponse {
  jobId: string;
  modelId: string;
  elapsedSeconds: number;
  triangles: number;
  textured: boolean;
  assetBase64: string;
  assetMime: string;
  assetFilename: string;
  backend: string;
  mcResolution: number;
  textureResolution: number;
  warning: string | null;
}

const candidate = modelCandidateById('triposr');
if (!candidate) throw new Error('TripoSR is missing from the model catalogue.');

export class TripoSRAdapter implements ModelAdapter {
  manifest = candidate.manifest;

  async generate(
    request: GenerationRequest,
    onProgress: (event: ProgressEvent) => void,
    signal: AbortSignal,
  ): Promise<GenerationResult> {
    if (!request.sourceBytes?.length) {
      throw new Error('The production adapter did not receive the source image bytes.');
    }

    const jobId = crypto.randomUUID();
    const unlisten = await listen<ProgressEvent>(`triposr-progress-${jobId}`, (event) => {
      onProgress(event.payload);
    });

    const cancel = () => {
      void invoke<boolean>('cancel_generation', { jobId }).catch(() => undefined);
    };
    signal.addEventListener('abort', cancel, { once: true });

    try {
      if (signal.aborted) throw new DOMException('Generation cancelled', 'AbortError');
      const response = await invoke<TripoResponse>('generate_triposr', {
        request: {
          jobId,
          quality: request.quality,
          sourceName: request.sourceName,
          sourceBytes: request.sourceBytes,
          backend: request.backend,
          backgroundRemoval: request.backgroundRemoval,
        },
      });
      if (signal.aborted) throw new DOMException('Generation cancelled', 'AbortError');

      return {
        jobId: response.jobId,
        modelId: response.modelId,
        quality: request.quality,
        elapsedSeconds: response.elapsedSeconds,
        triangles: response.triangles,
        textured: response.textured,
        assetBase64: response.assetBase64,
        assetMime: response.assetMime,
        assetFilename: response.assetFilename,
        warning: response.warning ?? undefined,
        metadata: {
          backend: response.backend,
          mcResolution: response.mcResolution,
          textureResolution: response.textureResolution,
          backgroundRemoval: request.backgroundRemoval,
          adapter: 'TripoSR M3',
        },
      };
    } catch (caught) {
      const text = caught instanceof Error ? caught.message : String(caught);
      if (signal.aborted || text.toLowerCase().includes('cancelled')) {
        throw new DOMException('Generation cancelled', 'AbortError');
      }
      throw caught instanceof Error ? caught : new Error(text);
    } finally {
      signal.removeEventListener('abort', cancel);
      unlisten();
    }
  }
}
