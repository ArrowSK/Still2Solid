import { invoke } from '@tauri-apps/api/core';
import type { HardwareProfile } from './types';

const fallback: HardwareProfile = {
  platform: 'Browser preview',
  architecture: navigator.userAgent.includes('Mac') ? 'arm64/unknown' : 'unknown',
  chip: 'Hardware probe available in the Tauri desktop runtime',
  memoryGb: 0,
  osVersion: 'Development preview',
  preferredBackend: 'Auto',
  accelerators: [],
  supportsMetal: false,
  supportsCuda: false,
};

export async function getHardwareProfile(): Promise<HardwareProfile> {
  try {
    return await invoke<HardwareProfile>('get_hardware_profile');
  } catch {
    return fallback;
  }
}
