import type {
  HardwareProfile,
  ModelAdapter,
  ProgressEvent,
  QualityPreset,
  RuntimeBackend,
  TimingContext,
  TimingProfileSummary,
  TimingRunSummary,
  TimingStageSummary,
} from './types';

const STORAGE_KEY = 'still2solid.timingProfiles.v1';
const SCHEMA = 1;
const MAX_SAMPLES_PER_PROFILE = 24;
const MAX_PROFILES = 40;

export interface TimingSampleRecord {
  completedAt: number;
  totalSeconds: number;
  resolvedBackend: string;
  accepted: boolean;
  stageSeconds: Record<string, number>;
}

export interface TimingProfileRecord {
  context: TimingContext;
  updatedAt: number;
  samples: TimingSampleRecord[];
}

interface TimingStore {
  schema: number;
  profiles: Record<string, TimingProfileRecord>;
}

function emptyStore(): TimingStore {
  return { schema: SCHEMA, profiles: {} };
}

function storageAvailable(): boolean {
  return typeof localStorage !== 'undefined';
}

function readStore(): TimingStore {
  if (!storageAvailable()) return emptyStore();
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return emptyStore();
    const parsed = JSON.parse(raw) as TimingStore;
    if (parsed.schema !== SCHEMA || !parsed.profiles || typeof parsed.profiles !== 'object') return emptyStore();
    return parsed;
  } catch {
    return emptyStore();
  }
}

function writeStore(store: TimingStore): void {
  if (!storageAvailable()) return;
  const profiles = Object.entries(store.profiles)
    .sort(([, a], [, b]) => b.updatedAt - a.updatedAt)
    .slice(0, MAX_PROFILES);
  localStorage.setItem(STORAGE_KEY, JSON.stringify({ schema: SCHEMA, profiles: Object.fromEntries(profiles) }));
}

function finitePositive(values: number[]): number[] {
  return values.filter((value) => Number.isFinite(value) && value > 0);
}

export function median(values: number[]): number {
  const sorted = finitePositive(values).sort((a, b) => a - b);
  if (!sorted.length) return 0;
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

function mean(values: number[]): number {
  const usable = finitePositive(values);
  if (!usable.length) return 0;
  return usable.reduce((sum, value) => sum + value, 0) / usable.length;
}

function variability(values: number[]): number {
  const usable = finitePositive(values);
  const center = median(usable);
  if (!center || usable.length < 2) return 0;
  return median(usable.map((value) => Math.abs(value - center))) / center;
}

function confidenceFor(sampleCount: number, variation: number): TimingProfileSummary['confidence'] {
  if (sampleCount < 2) return 'low';
  if (sampleCount < 6) return variation <= 0.4 ? 'medium' : 'low';
  if (variation <= 0.25) return 'high';
  if (variation <= 0.5) return 'medium';
  return 'low';
}

function fnv1a(value: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, '0');
}

export function hardwareTimingKey(hardware: HardwareProfile): string {
  const canonical = [
    hardware.platform,
    hardware.architecture,
    hardware.chip,
    hardware.memoryGb.toFixed(1),
    hardware.accelerators
      .map((accelerator) => `${accelerator.type}:${accelerator.name}:${accelerator.memoryGb ?? 'unknown'}:${accelerator.backend}`)
      .sort()
      .join(','),
  ].join('|');
  return `${hardware.platform}-${hardware.architecture}-${fnv1a(canonical)}`;
}

export function createTimingContext(
  hardware: HardwareProfile,
  adapter: ModelAdapter,
  quality: QualityPreset,
  backend: RuntimeBackend,
  backgroundRemoval: boolean,
): TimingContext {
  return {
    hardwareKey: hardwareTimingKey(hardware),
    modelId: adapter.manifest.id,
    modelVersion: adapter.manifest.version,
    quality,
    backend,
    backgroundRemoval,
  };
}

export function timingContextKey(context: TimingContext): string {
  return [
    context.hardwareKey,
    context.modelId,
    context.modelVersion,
    context.quality,
    context.backend,
    context.backgroundRemoval ? 'foreground' : 'raw',
  ].join('::');
}

export function deriveStageSeconds(trace: ProgressEvent[], totalSeconds: number): Record<string, number> {
  const ordered = trace
    .filter((event) => Number.isFinite(event.elapsedSeconds) && event.elapsedSeconds >= 0)
    .slice()
    .sort((a, b) => a.elapsedSeconds - b.elapsedSeconds);
  const stageIds = [...new Set(ordered.map((event) => event.stageId))];
  const result: Record<string, number> = {};
  let previousBoundary = 0;

  for (const stageId of stageIds) {
    const events = ordered.filter((event) => event.stageId === stageId);
    const first = events[0];
    const last = events[events.length - 1];
    let duration = last.elapsedSeconds - first.elapsedSeconds;

    if (duration <= 0.001) {
      duration = Math.max(0, first.elapsedSeconds - previousBoundary);
    }

    if (duration > 0.001 && Number.isFinite(duration)) result[stageId] = duration;
    previousBoundary = Math.max(previousBoundary, last.elapsedSeconds);
  }

  const measured = Object.values(result).reduce((sum, value) => sum + value, 0);
  if (measured > 0 && totalSeconds > measured * 1.35) {
    const finalStage = stageIds.at(-1);
    if (finalStage) result[finalStage] = (result[finalStage] ?? 0) + (totalSeconds - measured);
  }
  return result;
}

export function shouldAcceptTimingSample(existingAcceptedTotals: number[], totalSeconds: number): boolean {
  if (!Number.isFinite(totalSeconds) || totalSeconds < 0.25 || totalSeconds > 7200) return false;
  if (existingAcceptedTotals.length < 4) return true;
  const center = median(existingAcceptedTotals);
  if (!center) return true;
  return totalSeconds >= center / 3 && totalSeconds <= center * 3;
}

export function summarizeTimingProfile(key: string, record: TimingProfileRecord | undefined): TimingProfileSummary | null {
  if (!record) return null;
  const accepted = record.samples.filter((sample) => sample.accepted);
  if (!accepted.length) return {
    key,
    sampleCount: 0,
    confidence: 'low',
    medianTotalSeconds: 0,
    variability: 0,
    stages: [],
    recentRuns: record.samples.slice(-5).reverse().map(toRunSummary),
    updatedAt: record.updatedAt,
  };

  const totals = accepted.map((sample) => sample.totalSeconds);
  const allStageIds = [...new Set(accepted.flatMap((sample) => Object.keys(sample.stageSeconds)))];
  const stages: TimingStageSummary[] = allStageIds.map((stageId) => {
    const values = accepted.map((sample) => sample.stageSeconds[stageId]).filter((value) => value > 0);
    return {
      stageId,
      medianSeconds: median(values),
      meanSeconds: mean(values),
      sampleCount: values.length,
    };
  });
  const variation = variability(totals);

  return {
    key,
    sampleCount: accepted.length,
    confidence: confidenceFor(accepted.length, variation),
    medianTotalSeconds: median(totals),
    variability: variation,
    stages,
    recentRuns: record.samples.slice(-5).reverse().map(toRunSummary),
    updatedAt: record.updatedAt,
  };
}

function toRunSummary(sample: TimingSampleRecord): TimingRunSummary {
  return {
    completedAt: sample.completedAt,
    totalSeconds: sample.totalSeconds,
    resolvedBackend: sample.resolvedBackend,
    accepted: sample.accepted,
  };
}

export function loadTimingProfile(context: TimingContext): TimingProfileSummary | null {
  const key = timingContextKey(context);
  return summarizeTimingProfile(key, readStore().profiles[key]);
}

export function recordSuccessfulTiming(
  context: TimingContext,
  trace: ProgressEvent[],
  totalSeconds: number,
  resolvedBackend: string,
): TimingProfileSummary | null {
  const key = timingContextKey(context);
  const store = readStore();
  const current = store.profiles[key] ?? { context, updatedAt: 0, samples: [] };
  const effectiveTotal = Math.max(totalSeconds, trace.at(-1)?.elapsedSeconds ?? 0);
  const acceptedTotals = current.samples.filter((sample) => sample.accepted).map((sample) => sample.totalSeconds);
  const accepted = shouldAcceptTimingSample(acceptedTotals, effectiveTotal);

  current.context = context;
  current.updatedAt = Date.now();
  current.samples.push({
    completedAt: current.updatedAt,
    totalSeconds: effectiveTotal,
    resolvedBackend,
    accepted,
    stageSeconds: deriveStageSeconds(trace, effectiveTotal),
  });
  current.samples = current.samples.slice(-MAX_SAMPLES_PER_PROFILE);
  store.profiles[key] = current;
  writeStore(store);
  return summarizeTimingProfile(key, current);
}

export function clearTimingProfile(context: TimingContext): void {
  const key = timingContextKey(context);
  const store = readStore();
  delete store.profiles[key];
  writeStore(store);
}

export function estimateProgressFromTiming(
  event: ProgressEvent,
  profile: TimingProfileSummary | null,
  stageOrder: string[],
  secondsSinceEvent = 0,
): ProgressEvent {
  if (!profile || profile.sampleCount < 1 || !profile.stages.length) return event;

  const stageMap = new Map(profile.stages.map((stage) => [stage.stageId, stage.medianSeconds]));
  const currentIndex = stageOrder.indexOf(event.stageId);
  const currentDuration = stageMap.get(event.stageId) ?? 0;
  const expected = stageOrder.map((stageId) => stageMap.get(stageId) ?? 0);
  const expectedTotal = expected.reduce((sum, value) => sum + value, 0);
  if (currentIndex < 0 || currentDuration <= 0 || expectedTotal <= 0) return event;

  const isComplete = currentIndex === stageOrder.length - 1 && event.stageProgress >= 0.999;
  let stageProgress = event.stageProgress;
  if (!isComplete && event.stageProgress < 0.999 && secondsSinceEvent > 0) {
    const learnedAdvance = secondsSinceEvent / currentDuration;
    stageProgress = Math.min(0.96, Math.max(stageProgress, stageProgress + learnedAdvance * (1 - stageProgress)));
  }

  const completedBefore = expected.slice(0, currentIndex).reduce((sum, value) => sum + value, 0);
  const remainingAfter = expected.slice(currentIndex + 1).reduce((sum, value) => sum + value, 0);
  const weightedOverall = (completedBefore + stageProgress * currentDuration) / expectedTotal;
  const etaSeconds = isComplete ? 0 : Math.max(0, (1 - stageProgress) * currentDuration + remainingAfter);

  return {
    ...event,
    stageProgress,
    overallProgress: isComplete ? 1 : Math.max(0, Math.min(0.999, weightedOverall)),
    progressIsEstimated: true,
    etaSeconds,
    etaConfidence: profile.confidence,
  };
}
