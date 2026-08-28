import { mockManifest } from './mockAdapter';
import type { ModelCandidate, ModelManifest } from './types';

const reconstructionStages: ModelManifest['stages'] = [
  { id: 'prepare', label: 'Preparing image' },
  { id: 'isolate', label: 'Isolating object' },
  { id: 'load', label: 'Loading model' },
  { id: 'reconstruct', label: 'Reconstructing geometry' },
  { id: 'mesh', label: 'Extracting mesh' },
  { id: 'texture', label: 'Baking texture' },
  { id: 'preview', label: 'Preparing preview' },
];

export const modelCandidates: ModelCandidate[] = [
  {
    manifest: mockManifest,
    summary: 'Deterministic development adapter used to validate the Still2Solid workflow.',
    sourceLabel: 'Built into Still2Solid',
    availability: 'bundled',
    hardwareNotes: ['Runs everywhere the desktop application runs.', 'Does not perform AI inference.'],
    licenseNote: 'No external model weights or model licence apply.',
    runtimeAdapter: null,
  },
  {
    manifest: {
      id: 'triposr',
      name: 'TripoSR',
      family: 'TripoSR',
      version: 'HF 5b521936 · source 107cefdc',
      license: 'MIT',
      licenseStatus: 'verified-permissive',
      supportsTexture: true,
      supportsUv: true,
      supportsPbr: false,
      diskSizeMb: 1680,
      stages: reconstructionStages,
    },
    summary: 'Fast single-image reconstruction with a comparatively small model footprint and permissive licensing.',
    sourceLabel: 'VAST-AI-Research/TripoSR + stabilityai/TripoSR',
    availability: 'catalog-only',
    hardwareNotes: [
      'M3 pins upstream source commit 107cefdc244c39106fa830359024f6a2f1c78871.',
      'M3 pins model revision 5b521936b01fbe1890f6f9baed0254ab6351c04a and verifies its checkpoint SHA-256.',
      'Upstream documents about 6 GB VRAM for the default single-image path.',
      'M3 uses conservative extraction chunks and a CPU marching-cubes shim to avoid the upstream torchmcubes build on Apple Silicon.',
    ],
    licenseNote: 'The upstream repository states that source code and pretrained models are MIT licensed. Still2Solid stores the upstream licence with the installed source.',
    runtimeAdapter: 'triposr',
  },
  {
    manifest: {
      id: 'sf3d',
      name: 'Stable Fast 3D',
      family: 'SF3D',
      version: 'stabilityai/stable-fast-3d',
      license: 'Stability AI Community License',
      licenseStatus: 'conditional',
      supportsTexture: true,
      supportsUv: true,
      supportsPbr: true,
      diskSizeMb: 4020,
      stages: reconstructionStages,
    },
    summary: 'More game-ready reconstruction with UV unwrapping, delighting and material prediction.',
    sourceLabel: 'Hugging Face · stabilityai/stable-fast-3d',
    availability: 'gated',
    hardwareNotes: [
      'Upstream MPS support is experimental and was tested on an M1 Max with 64 GB unified memory.',
      'Upstream recommends CPU execution on Apple Silicon systems with less than 32 GB unified memory.',
      'CUDA and CPU execution are supported by the upstream project.',
    ],
    licenseNote: 'Gated access. Commercial use is subject to the Stability AI Community License, including its current revenue and registration terms.',
    runtimeAdapter: null,
  },
  {
    manifest: {
      id: 'trellis2-4b',
      name: 'TRELLIS.2 4B',
      family: 'TRELLIS.2',
      version: 'microsoft/TRELLIS.2-4B',
      license: 'MIT',
      licenseStatus: 'verified-permissive',
      supportsTexture: true,
      supportsUv: true,
      supportsPbr: false,
      diskSizeMb: 16200,
      stages: reconstructionStages,
    },
    summary: 'High-fidelity image-to-3D model with a much larger hardware and storage requirement.',
    sourceLabel: 'Hugging Face · microsoft/TRELLIS.2-4B',
    availability: 'catalog-only',
    hardwareNotes: [
      'Upstream currently requires Linux and an NVIDIA GPU with at least 24 GB VRAM.',
      'The published checkpoint is approximately 16.2 GB.',
      'Several runtime dependencies have licences separate from the model licence.',
    ],
    licenseNote: 'The model and main code are MIT licensed; runtime dependencies must be reviewed separately before distribution.',
    runtimeAdapter: null,
  },
];

export function modelCandidateById(id: string): ModelCandidate | undefined {
  return modelCandidates.find((candidate) => candidate.manifest.id === id);
}
