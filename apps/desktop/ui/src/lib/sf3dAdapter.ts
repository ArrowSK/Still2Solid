import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { modelCandidateById } from './modelCatalog';
import type {
  GenerationRequest,
  GenerationResult,
  ModelAdapter,
  ProgressEvent,
} from './types';

interface Sf3dResponse {
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

const candidate = modelCandidateById('sf3d');
if (!candidate) throw new Error('Stable Fast 3D is missing from the model catalogue.');
const sf3dManifest = candidate.manifest;

export class Sf3DAdapter implements ModelAdapter {
  manifest = sf3dManifest;

  async generate(
    request: GenerationRequest,
    onProgress: (event: ProgressEvent) => void,
    signal: AbortSignal,
  ): Promise<GenerationResult> {
    if (!request.sourceBytes?.length) {
      throw new Error('The production adapter did not receive the source image bytes.');
    }

    const jobId = crypto.randomUUID();
    const unlisten = await listen<ProgressEvent>(`sf3d-progress-${jobId}`, (event) => {
      onProgress(event.payload);
    });

    const cancel = () => {
      void invoke<boolean>('cancel_sf3d', { jobId }).catch(() => undefined);
    };
    signal.addEventListener('abort', cancel, { once: true });

    try {
      if (signal.aborted) throw new DOMException('Generation cancelled', 'AbortError');
      const response = await invoke<Sf3dResponse>('generate_sf3d', {
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
          adapter: 'Stable Fast 3D M8',
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
