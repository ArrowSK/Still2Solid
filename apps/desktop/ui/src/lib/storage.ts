import { invoke } from '@tauri-apps/api/core';

export interface StorageSummary {
  modelsBytes: number;
  cacheBytes: number;
  temporaryBytes: number;
  otherAppDataBytes: number;
  totalRemovableBytes: number;
  installedModelDirectories: number;
  nativeAvailable: boolean;
}

const emptySummary = (): StorageSummary => ({
  modelsBytes: 0,
  cacheBytes: 0,
  temporaryBytes: 0,
  otherAppDataBytes: 0,
  totalRemovableBytes: 0,
  installedModelDirectories: 0,
  nativeAvailable: false,
});

export async function getStorageSummary(): Promise<StorageSummary> {
  try {
    const summary = await invoke<Omit<StorageSummary, 'nativeAvailable'>>('get_storage_summary');
    return { ...summary, nativeAvailable: true };
  } catch {
    return emptySummary();
  }
}

export async function clearAppCache(): Promise<StorageSummary> {
  const summary = await invoke<Omit<StorageSummary, 'nativeAvailable'>>('clear_app_cache');
  return { ...summary, nativeAvailable: true };
}

export async function clearAppOwnedData(): Promise<StorageSummary> {
  const summary = await invoke<Omit<StorageSummary, 'nativeAvailable'>>('clear_app_owned_data');
  return { ...summary, nativeAvailable: true };
}

export function openApplicationsFolder(): Promise<void> {
  return invoke<void>('open_applications_folder');
}
