import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ModelInstallProgress, ModelRuntimeState } from './types';

export async function getModelRuntimeStates(): Promise<ModelRuntimeState[]> {
  try {
    return await invoke<ModelRuntimeState[]>('get_model_runtime_states');
  } catch {
    return [
      {
        modelId: 'triposr',
        status: 'unavailable',
        installed: false,
        verified: false,
        runtimeReady: false,
        canGenerate: false,
        detail: 'Production runtime management is available in the Tauri desktop build.',
        installedBytes: 0,
        sourceRevision: '107cefdc244c39106fa830359024f6a2f1c78871',
        weightSha256: '429e2c6b22a0923967459de24d67f05962b235f79cde6b032aa7ed2ffcd970ee',
        pythonVersion: null,
      },
    ];
  }
}

export function installModel(modelId: string): Promise<ModelRuntimeState> {
  return invoke<ModelRuntimeState>('install_model', { modelId });
}

export function uninstallModel(modelId: string): Promise<ModelRuntimeState> {
  return invoke<ModelRuntimeState>('uninstall_model', { modelId });
}

export function listenForInstallProgress(
  handler: (progress: ModelInstallProgress) => void,
): Promise<UnlistenFn> {
  return listen<ModelInstallProgress>('model-install-progress', (event) => handler(event.payload));
}
