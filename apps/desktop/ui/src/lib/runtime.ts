import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ModelInstallProgress, ModelRuntimeState } from './types';

const unavailableRuntime = (
  modelId: string,
  sourceRevision: string,
  weightSha256: string,
): ModelRuntimeState => ({
  modelId,
  status: 'unavailable',
  installed: false,
  verified: false,
  runtimeReady: false,
  canGenerate: false,
  detail: 'Production runtime management is available in the Tauri desktop build.',
  installedBytes: 0,
  sourceRevision,
  weightSha256,
  pythonVersion: null,
});

export async function getModelRuntimeStates(): Promise<ModelRuntimeState[]> {
  try {
    const [triposrStates, sf3d] = await Promise.all([
      invoke<ModelRuntimeState[]>('get_model_runtime_states'),
      invoke<ModelRuntimeState>('get_sf3d_runtime_state'),
    ]);
    return [...triposrStates.filter((runtime) => runtime.modelId !== 'sf3d'), sf3d];
  } catch {
    return [
      unavailableRuntime(
        'triposr',
        '107cefdc244c39106fa830359024f6a2f1c78871',
        '429e2c6b22a0923967459de24d67f05962b235f79cde6b032aa7ed2ffcd970ee',
      ),
      unavailableRuntime(
        'sf3d',
        'ff21fc491b4dc5314bf6734c7c0dabd86b5f5bb2',
        'a3416e1cf654e7d4f5e75f116cec2c3f0a14501a77d30c2f6068bbda178de388',
      ),
    ];
  }
}

export function installModel(modelId: string, modelUrl?: string): Promise<ModelRuntimeState> {
  return invoke<ModelRuntimeState>('install_model', { modelId, modelUrl: modelUrl?.trim() || null });
}

export function uninstallModel(modelId: string): Promise<ModelRuntimeState> {
  return invoke<ModelRuntimeState>('uninstall_model', { modelId });
}

export function installSf3d(hfToken: string, acceptedLicense: boolean): Promise<ModelRuntimeState> {
  return invoke<ModelRuntimeState>('install_sf3d', { hfToken, acceptedLicense });
}

export function uninstallSf3d(): Promise<ModelRuntimeState> {
  return invoke<ModelRuntimeState>('uninstall_sf3d');
}

export function listenForInstallProgress(
  handler: (progress: ModelInstallProgress) => void,
): Promise<UnlistenFn> {
  return listen<ModelInstallProgress>('model-install-progress', (event) => handler(event.payload));
}
