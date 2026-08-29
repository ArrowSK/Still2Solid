import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface UpdateInfo {
  available: boolean;
  currentVersion: string;
  latestVersion: string;
  releaseUrl: string;
  notes: string;
  downloadSize: number;
}

export interface DownloadedUpdate {
  version: string;
  path: string;
  sha256: string;
  releaseUrl: string;
}

export interface UpdateDownloadProgress {
  downloadedBytes: number;
  totalBytes: number;
  progress: number;
  message: string;
}

export async function checkForUpdates(): Promise<UpdateInfo> {
  return invoke<UpdateInfo>('check_for_updates');
}

export async function downloadUpdate(): Promise<DownloadedUpdate> {
  return invoke<DownloadedUpdate>('download_update');
}

export async function openUpdateInstaller(path: string): Promise<void> {
  return invoke<void>('open_update_installer', { path });
}

export function listenForUpdateProgress(
  handler: (progress: UpdateDownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<UpdateDownloadProgress>('update-download-progress', (event) => handler(event.payload));
}
