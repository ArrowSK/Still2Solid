import { modelCandidates } from './modelCatalog';
import type { HardwareProfile, ModelAssessment, ModelCandidate } from './types';

const LABELS: Record<ModelAssessment['compatibility'], string> = {
  recommended: 'Recommended',
  compatible: 'Compatible',
  slow: 'Compatible · slow path',
  'memory-constrained': 'Memory constrained',
  unsupported: 'Unsupported',
  'license-restricted': 'Licence restricted',
};

function assessment(
  modelId: string,
  compatibility: ModelAssessment['compatibility'],
  score: number,
  reasons: string[],
  caveats: string[] = [],
): ModelAssessment {
  return { modelId, compatibility, score, label: LABELS[compatibility], reasons, caveats };
}

function isAppleSilicon(hardware: HardwareProfile): boolean {
  return hardware.platform === 'macos' && ['aarch64', 'arm64'].includes(hardware.architecture.toLowerCase());
}

function nvidiaMemory(hardware: HardwareProfile): number | null {
  const accelerator = hardware.accelerators.find((item) => item.type === 'nvidia');
  return accelerator?.memoryGb ?? null;
}

function hasNvidia(hardware: HardwareProfile): boolean {
  return hardware.accelerators.some((item) => item.type === 'nvidia') || hardware.supportsCuda;
}

function assessTripoSr(hardware: HardwareProfile): ModelAssessment {
  const vram = nvidiaMemory(hardware);

  if (hasNvidia(hardware)) {
    if (vram !== null && vram < 6) {
      return assessment('triposr', 'memory-constrained', 45, [
        `Detected NVIDIA memory: ${vram.toFixed(1)} GB.`,
        'TripoSR documents about 6 GB VRAM for its default single-image path.',
      ]);
    }
    return assessment('triposr', 'compatible', 96, [
      'CUDA-capable NVIDIA hardware detected.',
      vram === null ? 'GPU memory could not be measured, so the 6 GB upstream reference cannot be verified.' : `${vram.toFixed(1)} GB NVIDIA memory clears the documented ~6 GB reference.`,
      'MIT licensing keeps the model suitable for an unrestricted default candidate.',
    ]);
  }

  if (isAppleSilicon(hardware)) {
    if (hardware.memoryGb >= 16) {
      return assessment('triposr', 'compatible', 90, [
        `${hardware.memoryGb.toFixed(1)} GB unified memory leaves reasonable headroom above the upstream ~6 GB accelerator footprint.`,
        'TripoSR is the smallest permissively licensed production candidate in the M2 catalogue.',
      ], [
        'The upstream TripoSR project does not document Apple MPS as a certified path; M3 must validate the worker backend before enabling inference.',
      ]);
    }
    if (hardware.memoryGb >= 8) {
      return assessment('triposr', 'memory-constrained', 50, [
        `${hardware.memoryGb.toFixed(1)} GB unified memory is close to the upstream ~6 GB accelerator footprint before macOS and application overhead.`,
      ], ['Still2Solid will keep Mock3D active until a production worker passes memory validation.']);
    }
    return assessment('triposr', 'unsupported', 10, ['Less than 8 GB system memory does not provide a safe margin for the planned production worker.']);
  }

  if (hardware.memoryGb >= 16) {
    return assessment('triposr', 'slow', 68, [
      `${hardware.memoryGb.toFixed(1)} GB system memory is sufficient for a CPU-oriented fallback evaluation.`,
      'No supported accelerated backend was detected.',
    ], ['Expected performance is materially slower than the documented CUDA path.']);
  }
  if (hardware.memoryGb >= 8) {
    return assessment('triposr', 'memory-constrained', 40, ['No supported accelerator was detected and system memory is limited.']);
  }
  return assessment('triposr', 'unsupported', 10, ['No suitable accelerator or memory headroom was detected.']);
}

function assessSf3d(hardware: HardwareProfile): ModelAssessment {
  const vram = nvidiaMemory(hardware);

  if (isAppleSilicon(hardware)) {
    if (hardware.memoryGb >= 32) {
      return assessment('sf3d', 'compatible', 88, [
        `${hardware.memoryGb.toFixed(1)} GB unified memory meets the upstream 32 GB threshold above which MPS is not discouraged.`,
        'Apple Silicon MPS support exists upstream.',
      ], [
        'MPS support is explicitly experimental upstream.',
        'The model is gated and uses the conditional Stability AI Community License.',
      ]);
    }
    if (hardware.memoryGb > 0) {
      return assessment('sf3d', 'memory-constrained', 42, [
        `${hardware.memoryGb.toFixed(1)} GB unified memory is below the upstream 32 GB MPS guidance.`,
        'Upstream recommends CPU execution below 32 GB unified memory.',
      ], ['CPU fallback may work but is not a preferred interactive path.', 'The model is gated and conditionally licensed.']);
    }
  }

  if (hasNvidia(hardware)) {
    if (vram !== null && vram < 6) {
      return assessment('sf3d', 'memory-constrained', 48, [`Detected NVIDIA memory: ${vram.toFixed(1)} GB.`], [
        'SF3D does not publish a simple minimum-VRAM figure comparable to TripoSR, so M2 uses a conservative floor.',
        'The model is gated and conditionally licensed.',
      ]);
    }
    return assessment('sf3d', 'compatible', 92, [
      'CUDA-capable NVIDIA hardware detected.',
      'The upstream project supports CUDA execution.',
    ], ['The model is gated and conditionally licensed, so it is never selected automatically in M2.']);
  }

  if (hardware.memoryGb >= 32) {
    return assessment('sf3d', 'slow', 58, ['CPU execution is supported upstream and sufficient system memory is available.'], [
      'This is expected to be substantially slower than an accelerated path.',
      'The model is gated and conditionally licensed.',
    ]);
  }
  if (hardware.memoryGb > 0) {
    return assessment('sf3d', 'memory-constrained', 30, ['No preferred accelerator was detected and system memory is below 32 GB.']);
  }
  return assessment('sf3d', 'compatible', 35, ['Hardware details are unavailable in browser preview.'], ['Run the Tauri desktop build for a reliable assessment.']);
}

function assessTrellis2(hardware: HardwareProfile): ModelAssessment {
  const vram = nvidiaMemory(hardware);
  if (hardware.platform !== 'linux') {
    return assessment('trellis2-4b', 'unsupported', 5, ['Upstream currently documents Linux as the supported operating system.']);
  }
  if (!hasNvidia(hardware)) {
    return assessment('trellis2-4b', 'unsupported', 5, ['Upstream requires an NVIDIA GPU.']);
  }
  if (vram === null) {
    return assessment('trellis2-4b', 'memory-constrained', 35, ['NVIDIA hardware is present, but VRAM could not be measured.'], ['Upstream requires at least 24 GB VRAM.']);
  }
  if (vram < 24) {
    return assessment('trellis2-4b', 'memory-constrained', 35, [`${vram.toFixed(1)} GB VRAM is below the upstream 24 GB minimum.`]);
  }
  return assessment('trellis2-4b', 'compatible', 120, [
    `Linux with ${vram.toFixed(1)} GB NVIDIA VRAM meets the published minimum.`,
    'The model and main code are MIT licensed.',
  ], ['The checkpoint is about 16.2 GB and its runtime has separately licensed dependencies.']);
}

function assessBase(candidate: ModelCandidate, hardware: HardwareProfile): ModelAssessment {
  switch (candidate.manifest.id) {
    case 'mock3d':
      return assessment('mock3d', 'compatible', 1, ['Bundled deterministic adapter; no external weights or accelerator required.']);
    case 'triposr':
      return assessTripoSr(hardware);
    case 'sf3d':
      return assessSf3d(hardware);
    case 'trellis2-4b':
      return assessTrellis2(hardware);
    default:
      return assessment(candidate.manifest.id, 'unsupported', 0, ['No compatibility policy is defined for this model.']);
  }
}

export function assessModels(
  hardware: HardwareProfile,
  candidates: ModelCandidate[] = modelCandidates,
): ModelAssessment[] {
  const base = candidates.map((candidate) => assessBase(candidate, hardware));
  const hardwareKnown = hardware.memoryGb > 0 || hardware.accelerators.length > 0;

  const eligible = base
    .filter((item) => item.modelId !== 'mock3d')
    .filter((item) => item.compatibility === 'compatible' || item.compatibility === 'slow')
    .filter((item) => {
      const candidate = candidates.find((model) => model.manifest.id === item.modelId);
      return candidate?.manifest.licenseStatus === 'verified-permissive' && candidate.availability !== 'gated';
    })
    .sort((a, b) => b.score - a.score);

  const recommendedId = hardwareKnown ? eligible[0]?.modelId : undefined;

  return base.map((item) =>
    item.modelId === recommendedId
      ? { ...item, compatibility: 'recommended', label: LABELS.recommended }
      : item,
  );
}

export function recommendedProductionModel(assessments: ModelAssessment[]): ModelAssessment | undefined {
  return assessments.find((item) => item.compatibility === 'recommended');
}

export function compatibilityLabel(value: ModelAssessment['compatibility']): string {
  return LABELS[value];
}
