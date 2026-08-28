export type QualityPreset = 'fast' | 'standard' | 'best';

export type Compatibility =
  | 'recommended'
  | 'compatible'
  | 'slow'
  | 'memory-constrained'
  | 'unsupported'
  | 'license-restricted';

export interface HardwareAccelerator {
  type: 'apple-unified' | 'nvidia' | 'other';
  name: string;
  memoryGb: number | null;
  backend: 'metal' | 'cuda' | 'other';
}

export interface HardwareProfile {
  platform: string;
  architecture: string;
  chip: string;
  memoryGb: number;
  osVersion: string;
  preferredBackend: string;
  accelerators: HardwareAccelerator[];
  supportsMetal: boolean;
  supportsCuda: boolean;
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

export type ModelAvailability = 'bundled' | 'catalog-only' | 'gated';

export interface ModelCandidate {
  manifest: ModelManifest;
  summary: string;
  sourceLabel: string;
  availability: ModelAvailability;
  hardwareNotes: string[];
  licenseNote: string;
  runtimeAdapter: 'triposr' | 'sf3d' | null;
}

export interface ModelAssessment {
  modelId: string;
  compatibility: Compatibility;
  score: number;
  label: string;
  reasons: string[];
  caveats: string[];
}

export type RuntimeBackend = 'auto' | 'metal' | 'cuda' | 'cpu';

export interface GenerationRequest {
  quality: QualityPreset;
  sourceName: string;
  sourceSizeBytes: number;
  sourceBytes?: number[];
  backend: RuntimeBackend;
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

export interface TimingContext {
  hardwareKey: string;
  modelId: string;
  modelVersion: string;
  quality: QualityPreset;
  backend: RuntimeBackend;
  backgroundRemoval: boolean;
}

export interface TimingStageSummary {
  stageId: string;
  medianSeconds: number;
  meanSeconds: number;
  sampleCount: number;
}

export interface TimingRunSummary {
  completedAt: number;
  totalSeconds: number;
  resolvedBackend: string;
  accepted: boolean;
}

export interface TimingProfileSummary {
  key: string;
  sampleCount: number;
  confidence: 'low' | 'medium' | 'high';
  medianTotalSeconds: number;
  variability: number;
  stages: TimingStageSummary[];
  recentRuns: TimingRunSummary[];
  updatedAt: number;
}

export interface GenerationResult {
  jobId: string;
  modelId: string;
  quality: QualityPreset;
  elapsedSeconds: number;
  triangles: number;
  textured: boolean;
  metadata: Record<string, string | number | boolean>;
  assetBase64?: string;
  assetMime?: string;
  assetFilename?: string;
  warning?: string;
}

export interface ModelAdapter {
  manifest: ModelManifest;
  generate(
    request: GenerationRequest,
    onProgress: (event: ProgressEvent) => void,
    signal: AbortSignal,
  ): Promise<GenerationResult>;
}

export type ModelRuntimeStatus = 'not-installed' | 'installing' | 'ready' | 'broken' | 'unavailable';

export interface ModelRuntimeState {
  modelId: string;
  status: ModelRuntimeStatus;
  installed: boolean;
  verified: boolean;
  runtimeReady: boolean;
  canGenerate: boolean;
  detail: string;
  installedBytes: number;
  sourceRevision: string;
  weightSha256: string;
  pythonVersion: string | null;
}

export interface ModelInstallProgress {
  modelId: string;
  stage: string;
  stageProgress: number;
  overallProgress: number;
  message: string;
  bytesDownloaded: number | null;
  bytesTotal: number | null;
}
