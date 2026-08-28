export type QualityPreset = 'fast' | 'standard' | 'best';

export type Compatibility =
  | 'recommended'
  | 'compatible'
  | 'slow'
  | 'memory-constrained'
  | 'unsupported'
  | 'license-restricted';

export interface HardwareProfile {
  platform: string;
  architecture: string;
  chip: string;
  memoryGb: number;
  osVersion: string;
  preferredBackend: string;
}

export interface ModelManifest {
  id: string;
  name: string;
  family: string;
  version: string;
  license: string;
  licenseStatus: 'verified-permissive' | 'conditional' | 'restricted' | 'unknown' | 'not-applicable';
  supportsTexture: boolean;
  supportsUv: boolean;
  supportsPbr: boolean;
  diskSizeMb: number;
  stages: Array<{ id: string; label: string }>;
}

export interface GenerationRequest {
  quality: QualityPreset;
  sourceName: string;
  sourceSizeBytes: number;
  backend: 'auto' | 'metal' | 'cpu';
  backgroundRemoval: boolean;
}

export interface ProgressEvent {
  stageId: string;
  stageName: string;
  stageProgress: number;
  overallProgress: number;
  progressIsEstimated: boolean;
  elapsedSeconds: number;
  etaSeconds: number;
  etaConfidence: 'low' | 'medium' | 'high';
  statusMessage: string;
}

export interface GenerationResult {
  jobId: string;
  modelId: string;
  quality: QualityPreset;
  elapsedSeconds: number;
  triangles: number;
  textured: boolean;
  metadata: Record<string, string | number | boolean>;
}

export interface ModelAdapter {
  manifest: ModelManifest;
  generate(
    request: GenerationRequest,
    onProgress: (event: ProgressEvent) => void,
    signal: AbortSignal,
  ): Promise<GenerationResult>;
}
