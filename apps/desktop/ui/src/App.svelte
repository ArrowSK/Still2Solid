<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import ModelManager from './lib/ModelManager.svelte';
  import ModelViewer from './lib/ModelViewer.svelte';
  import { getHardwareProfile } from './lib/hardware';
  import { modelCandidateById } from './lib/modelCatalog';
  import { Mock3DAdapter } from './lib/mockAdapter';
  import { assessModels, recommendedProductionModel } from './lib/recommendation';
  import { getModelRuntimeStates } from './lib/runtime';
  import {
    clearTimingProfile,
    createTimingContext,
    estimateProgressFromTiming,
    loadTimingProfile,
    recordSuccessfulTiming,
    timingContextKey,
  } from './lib/timing';
  import { TripoSRAdapter } from './lib/triposrAdapter';
  import type {
    GenerationResult,
    HardwareProfile,
    ModelAdapter,
    ModelAssessment,
    ModelRuntimeState,
    ProgressEvent,
    QualityPreset,
    RuntimeBackend,
    TimingContext,
    TimingProfileSummary,
  } from './lib/types';

  const mockAdapter = new Mock3DAdapter();
  const triposrAdapter = new TripoSRAdapter();
  const qualities: Array<{ id: QualityPreset; title: string; detail: string }> = [
    { id: 'fast', title: 'Fast', detail: 'Lowest memory pressure and quickest mesh' },
    { id: 'standard', title: 'Standard', detail: 'Balanced geometry and texture detail' },
    { id: 'best', title: 'Best', detail: 'Higher-detail extraction and texture' },
  ];

  let quality: QualityPreset = 'standard';
  let sourceFile: File | null = null;
  let sourceUrl = '';
  let previewModelUrl = '';
  let previewFilename = 'still2solid.glb';
  let hardware: HardwareProfile = {
    platform: 'Detecting…',
    architecture: '—',
    chip: 'Detecting hardware…',
    memoryGb: 0,
    osVersion: '—',
    preferredBackend: 'Auto',
    accelerators: [],
    supportsMetal: false,
    supportsCuda: false,
  };
  let progress: ProgressEvent | null = null;
  let result: GenerationResult | null = null;
  let generating = false;
  let error = '';
  let controller: AbortController | null = null;
  let jobAdapter: ModelAdapter | null = null;
  let advanced = false;
  let backend: RuntimeBackend = 'auto';
  let backgroundRemoval = true;
  let wireframe = false;
  let showGrid = true;
  let fileInput: HTMLInputElement;
  let modelManagerOpen = false;
  let preferredProductionModelId = '';
  let modelAssessments: ModelAssessment[] = [];
  let runtimeStates: ModelRuntimeState[] = [];
  let activeAdapter: ModelAdapter = mockAdapter;

  let timingRevision = 0;
  let currentTimingContext: TimingContext;
  let currentTimingKey = '';
  let currentTimingProfile: TimingProfileSummary | null = null;
  let jobTimingProfile: TimingProfileSummary | null = null;
  let rawProgressTrace: ProgressEvent[] = [];
  let progressReceivedAt = 0;
  let clockNow = 0;
  let clockTimer: ReturnType<typeof setInterval> | null = null;

  $: modelAssessments = assessModels(hardware);
  $: automaticRecommendation = recommendedProductionModel(modelAssessments);
  $: preferredAssessment = modelAssessments.find((item) => item.modelId === preferredProductionModelId);
  $: preferredIsUsable = preferredAssessment && preferredAssessment.compatibility !== 'unsupported';
  $: productionAssessment = preferredIsUsable ? preferredAssessment : automaticRecommendation;
  $: productionCandidate = productionAssessment ? modelCandidateById(productionAssessment.modelId) : undefined;
  $: triposrRuntime = runtimeStates.find((runtime) => runtime.modelId === 'triposr');
  $: triposrAssessment = modelAssessments.find((assessment) => assessment.modelId === 'triposr');
  $: explicitTripo = preferredProductionModelId === 'triposr' && triposrAssessment?.compatibility !== 'unsupported';
  $: automaticTripo = !preferredProductionModelId && automaticRecommendation?.modelId === 'triposr';
  $: activeAdapter = triposrRuntime?.canGenerate && (explicitTripo || automaticTripo) ? triposrAdapter : mockAdapter;

  $: currentTimingContext = createTimingContext(hardware, activeAdapter, quality, backend, backgroundRemoval);
  $: currentTimingKey = timingContextKey(currentTimingContext);
  $: {
    timingRevision;
    currentTimingKey;
    currentTimingProfile = loadTimingProfile(currentTimingContext);
  }
  $: secondsSinceProgress = generating && progress && progressReceivedAt
    ? Math.max(0, (clockNow - progressReceivedAt) / 1000)
    : 0;
  $: displayProgress = progress
    ? estimateProgressFromTiming(
        progress,
        jobTimingProfile,
        (jobAdapter ?? activeAdapter).manifest.stages.map((stage) => stage.id),
        secondsSinceProgress,
      )
    : null;

  onMount(async () => {
    preferredProductionModelId = localStorage.getItem('still2solid.preferredProductionModel') ?? '';
    const [detectedHardware, detectedRuntimes] = await Promise.all([
      getHardwareProfile(),
      getModelRuntimeStates(),
    ]);
    hardware = detectedHardware;
    runtimeStates = detectedRuntimes;
    clockNow = performance.now();
    clockTimer = setInterval(() => {
      if (generating) clockNow = performance.now();
    }, 250);
  });

  onDestroy(() => {
    controller?.abort();
    if (clockTimer) clearInterval(clockTimer);
    if (sourceUrl) URL.revokeObjectURL(sourceUrl);
    if (previewModelUrl) URL.revokeObjectURL(previewModelUrl);
  });

  function clearPreviewModel() {
    if (previewModelUrl) URL.revokeObjectURL(previewModelUrl);
    previewModelUrl = '';
    previewFilename = 'still2solid.glb';
  }

  function selectFile(file: File | undefined) {
    if (!file) return;
    error = '';
    if (!file.type.startsWith('image/')) {
      error = 'Choose an image file (JPEG, PNG, HEIC or WebP).';
      return;
    }
    if (sourceUrl) URL.revokeObjectURL(sourceUrl);
    sourceFile = file;
    sourceUrl = URL.createObjectURL(file);
    clearPreviewModel();
    result = null;
    progress = null;
  }

  function onDrop(event: DragEvent) {
    event.preventDefault();
    selectFile(event.dataTransfer?.files?.[0]);
  }

  function decodeGeneratedAsset(generated: GenerationResult) {
    clearPreviewModel();
    if (!generated.assetBase64) return;
    const binary = atob(generated.assetBase64);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
    const blob = new Blob([bytes], { type: generated.assetMime ?? 'model/gltf-binary' });
    previewModelUrl = URL.createObjectURL(blob);
    const sourceBase = sourceFile?.name.replace(/\.[^.]+$/, '') || 'still2solid-model';
    previewFilename = `${sourceBase}-still2solid.glb`;
  }

  async function prepareProductionImage(): Promise<number[]> {
    if (!sourceFile || !sourceUrl) return [];
    const image = new Image();
    image.src = sourceUrl;
    await image.decode();
    const maxDimension = 2048;
    const scale = Math.min(1, maxDimension / Math.max(image.naturalWidth, image.naturalHeight));
    const canvas = document.createElement('canvas');
    canvas.width = Math.max(1, Math.round(image.naturalWidth * scale));
    canvas.height = Math.max(1, Math.round(image.naturalHeight * scale));
    const context = canvas.getContext('2d', { alpha: true });
    if (!context) throw new Error('Could not prepare the source image for local inference.');
    context.drawImage(image, 0, 0, canvas.width, canvas.height);
    const blob = await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob((value) => value ? resolve(value) : reject(new Error('Could not normalize the source image.')), 'image/png');
    });
    return Array.from(new Uint8Array(await blob.arrayBuffer()));
  }

  async function generate() {
    if (!sourceFile || generating) return;
    error = '';
    result = null;
    progress = null;
    clearPreviewModel();
    generating = true;
    controller = new AbortController();
    const adapterForJob = activeAdapter;
    const contextForJob = createTimingContext(hardware, adapterForJob, quality, backend, backgroundRemoval);
    jobAdapter = adapterForJob;
    jobTimingProfile = adapterForJob.manifest.id === 'triposr' ? loadTimingProfile(contextForJob) : null;
    rawProgressTrace = [];
    progressReceivedAt = 0;

    try {
      const isProduction = adapterForJob.manifest.id === 'triposr';
      const sourceBytes = isProduction ? await prepareProductionImage() : undefined;
      const generated = await adapterForJob.generate(
        {
          quality,
          sourceName: isProduction ? 'input.png' : sourceFile.name,
          sourceSizeBytes: sourceFile.size,
          sourceBytes,
          backend,
          backgroundRemoval,
        },
        (event) => {
          rawProgressTrace.push(event);
          progress = event;
          progressReceivedAt = performance.now();
          clockNow = progressReceivedAt;
        },
        controller.signal,
      );
      result = generated;
      decodeGeneratedAsset(generated);

      if (generated.modelId === 'triposr') {
        const resolvedBackend = typeof generated.metadata.backend === 'string'
          ? generated.metadata.backend
          : backend;
        recordSuccessfulTiming(
          contextForJob,
          rawProgressTrace,
          generated.elapsedSeconds,
          String(resolvedBackend),
        );
        timingRevision += 1;
      }
    } catch (caught) {
      if (caught instanceof DOMException && caught.name === 'AbortError') error = 'Generation cancelled.';
      else error = caught instanceof Error ? caught.message : String(caught || 'Generation failed.');
    } finally {
      generating = false;
      controller = null;
      jobAdapter = null;
      jobTimingProfile = null;
    }
  }

  function reset() {
    controller?.abort();
    sourceFile = null;
    result = null;
    progress = null;
    error = '';
    clearPreviewModel();
    if (sourceUrl) URL.revokeObjectURL(sourceUrl);
    sourceUrl = '';
  }

  function clearCurrentTiming() {
    clearTimingProfile(currentTimingContext);
    timingRevision += 1;
  }

  function formatSeconds(value: number): string {
    if (value < 10) return `${value.toFixed(1)} s`;
    return `${Math.round(value)} s`;
  }

  function formatRunTime(timestamp: number): string {
    return new Date(timestamp).toLocaleString([], {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function stageLabel(stageId: string): string {
    return activeAdapter.manifest.stages.find((stage) => stage.id === stageId)?.label ?? stageId;
  }

  const stageState = (id: string) => {
    if (!progress) return 'pending';
    const adapterForStages = jobAdapter ?? activeAdapter;
    const activeIndex = adapterForStages.manifest.stages.findIndex((stage) => stage.id === progress?.stageId);
    const index = adapterForStages.manifest.stages.findIndex((stage) => stage.id === id);
    if (index < activeIndex) return 'done';
    if (index === activeIndex) return progress.stageProgress >= 1 ? 'done' : 'active';
    return 'pending';
  };
</script>

<svelte:head><title>Still2Solid · M5</title></svelte:head>

<header class="topbar">
  <div>
    <div class="eyebrow">LOCAL IMAGE → 3D</div>
    <h1>Still2Solid</h1>
  </div>
  <div class="top-actions">
    <button type="button" class="secondary model-manager-button" on:click={() => (modelManagerOpen = true)}>Models</button>
    <div class="milestone">M5 · Asset exports</div>
  </div>
</header>

<main class="page">
  {#if !sourceFile}
    <section
      class="drop-zone"
      role="button"
      tabindex="0"
      on:dragover|preventDefault
      on:drop={onDrop}
      on:click={() => fileInput.click()}
      on:keydown={(event) => (event.key === 'Enter' || event.key === ' ') && fileInput.click()}
    >
      <div class="drop-icon">3D</div>
      <h2>Drop an image</h2>
      <p>or choose one from this computer. The file stays local.</p>
      <button type="button" class="primary">Choose image…</button>
      <input bind:this={fileInput} class="visually-hidden" type="file" accept="image/jpeg,image/png,image/webp,image/heic,image/heif" on:change={(event) => selectFile(event.currentTarget.files?.[0])} />
      <button type="button" class="model-hint" on:click|stopPropagation={() => (modelManagerOpen = true)}>
        {#if activeAdapter.manifest.id === 'triposr'}
          <span>Ready for local production inference</span>
          <strong>TripoSR</strong>
          <small>Verified runtime installed · timing improves automatically with successful runs</small>
        {:else if productionCandidate}
          <span>{preferredIsUsable ? 'Preferred production model' : 'Recommended for this computer'}</span>
          <strong>{productionCandidate.manifest.name}</strong>
          <small>{triposrRuntime?.installed ? 'Runtime not selected or not ready' : 'Open Model Manager to install a runtime'}</small>
        {:else}
          <span>Production model</span>
          <strong>Mock3D fallback is active</strong>
          <small>Open Model Manager for the hardware assessment and experimental install option</small>
        {/if}
      </button>
    </section>
  {:else}
    <section class="workspace-grid">
      <div class="source-card">
        <img src={sourceUrl} alt="Selected source" />
        <div class="source-meta">
          <div><strong>{sourceFile.name}</strong><span>{(sourceFile.size / 1024 / 1024).toFixed(1)} MB</span></div>
          <button type="button" class="text-button" on:click={reset} disabled={generating}>Change image</button>
        </div>
      </div>

      <div class="control-card">
        <div class="control-row">
          <div>
            <span class="control-label">Active adapter</span>
            <strong>{activeAdapter.manifest.name} <span class="badge">{activeAdapter.manifest.id === 'triposr' ? 'Production adapter' : 'Development fallback'}</span></strong>
          </div>
          <details>
            <summary>Why this model?</summary>
            <div class="explanation">
              {#if activeAdapter.manifest.id === 'triposr'}
                <p><strong>TripoSR is installed, checksum-verified and selected.</strong> Each generation runs in a one-shot isolated local process and unloads when it finishes.</p>
                <p>M4 timing remains local and M5 keeps each successful production GLB as the canonical master for preview and non-destructive exports.</p>
              {:else}
                <p><strong>Mock3D is the safe fallback.</strong> A production adapter is used only after its runtime is installed, verified and selected.</p>
                {#if triposrRuntime}<p>{triposrRuntime.detail}</p>{/if}
              {/if}
              <p>Open Model Manager for hardware, licence and installation details.</p>
            </div>
          </details>
        </div>

        <button type="button" class="recommendation-strip" on:click={() => (modelManagerOpen = true)}>
          <span>{activeAdapter.manifest.id === 'triposr' ? 'Production runtime' : preferredIsUsable ? 'Preferred candidate' : 'Production recommendation'}</span>
          <strong>{activeAdapter.manifest.id === 'triposr' ? 'TripoSR · ready' : productionCandidate?.manifest.name ?? 'No safe recommendation'}</strong>
          <small>{activeAdapter.manifest.id === 'triposr' ? 'Verified local install' : productionAssessment?.label ?? 'Inspect hardware constraints'}</small>
        </button>

        <fieldset>
          <legend>Quality</legend>
          <div class="quality-grid">
            {#each qualities as preset}
              <button type="button" class:active={quality === preset.id} on:click={() => (quality = preset.id)} disabled={generating}>
                <strong>{preset.title}</strong><span>{preset.detail}</span>
              </button>
            {/each}
          </div>
        </fieldset>

        <button type="button" class="advanced-toggle" on:click={() => (advanced = !advanced)} aria-expanded={advanced}>
          <span>Advanced</span><span>{advanced ? 'Hide' : 'Show'}</span>
        </button>

        {#if advanced}
          <div class="advanced-panel">
            <label>Backend
              <select bind:value={backend} disabled={generating}>
                <option value="auto">Auto</option>
                <option value="metal">Metal / MPS</option>
                <option value="cuda">CUDA</option>
                <option value="cpu">CPU</option>
              </select>
            </label>
            <label class="check"><input type="checkbox" bind:checked={backgroundRemoval} disabled={generating} /> Isolate foreground object</label>
            <div class="diagnostic"><span>Detected</span><strong>{hardware.platform} · {hardware.architecture}</strong></div>
            <div class="diagnostic"><span>Memory</span><strong>{hardware.memoryGb ? `${hardware.memoryGb.toFixed(1)} GB` : 'Unavailable in browser preview'}</strong></div>
            <div class="diagnostic"><span>Preferred backend</span><strong>{hardware.preferredBackend}</strong></div>
            <div class="diagnostic"><span>Accelerator</span><strong>{hardware.accelerators[0]?.name ?? 'None detected'}</strong></div>

            {#if activeAdapter.manifest.id === 'triposr'}
              <section class="timing-card" aria-label="Local timing profile">
                <div class="timing-heading">
                  <div>
                    <span>Local timing profile</span>
                    {#if currentTimingProfile?.sampleCount}
                      <strong>{currentTimingProfile.sampleCount} learned run{currentTimingProfile.sampleCount === 1 ? '' : 's'} · {currentTimingProfile.confidence} confidence</strong>
                    {:else}
                      <strong>Learning starts after the first successful run</strong>
                    {/if}
                  </div>
                  {#if currentTimingProfile?.sampleCount}
                    <button type="button" class="text-button" on:click={clearCurrentTiming} disabled={generating}>Reset</button>
                  {/if}
                </div>

                {#if currentTimingProfile?.sampleCount}
                  <div class="timing-summary">
                    <span>Median total <strong>{formatSeconds(currentTimingProfile.medianTotalSeconds)}</strong></span>
                    <span>Variation <strong>{Math.round(currentTimingProfile.variability * 100)}%</strong></span>
                  </div>
                  <div class="timing-stages">
                    {#each currentTimingProfile.stages as stage}
                      <div><span>{stageLabel(stage.stageId)}</span><strong>{formatSeconds(stage.medianSeconds)}</strong></div>
                    {/each}
                  </div>
                  {#if currentTimingProfile.recentRuns.length}
                    <div class="timing-history">
                      <span class="timing-caption">Recent successful completions</span>
                      {#each currentTimingProfile.recentRuns as run}
                        <div>
                          <span>{formatRunTime(run.completedAt)} · {run.resolvedBackend}</span>
                          <strong class:excluded={!run.accepted}>{formatSeconds(run.totalSeconds)}{run.accepted ? '' : ' · excluded'}</strong>
                        </div>
                      {/each}
                    </div>
                  {/if}
                {:else}
                  <p class="timing-empty">No cloud data or global benchmark is used. Still2Solid will learn only from completed generations on this hardware and this exact setting combination.</p>
                {/if}
              </section>
            {/if}
          </div>
        {/if}

        <button type="button" class="primary generate" on:click={generate} disabled={generating || !sourceFile}>
          {generating ? 'Generating…' : activeAdapter.manifest.id === 'triposr' ? 'Generate 3D locally' : 'Generate Mock3D'}
        </button>
      </div>
    </section>
  {/if}

  {#if error}
    <div class="message" role="status">{error}</div>
  {/if}

  {#if generating && displayProgress}
    <section class="progress-card" aria-live="polite">
      <div class="progress-heading">
        <div><span class="eyebrow">{jobAdapter?.manifest.id === 'triposr' ? 'LOCAL PRODUCTION GENERATION' : 'MOCK GENERATION'}</span><h2>{displayProgress.stageName}</h2></div>
        <strong>{Math.round(displayProgress.overallProgress * 100)}%</strong>
      </div>
      <div class="bar"><div style={`width:${displayProgress.overallProgress * 100}%`}></div></div>
      <div class="eta">
        <span>{displayProgress.statusMessage}</span>
        {#if displayProgress.etaSeconds > 0.5}
          <span>About {Math.ceil(displayProgress.etaSeconds)} s remaining · {displayProgress.etaConfidence} confidence</span>
        {:else if jobAdapter?.manifest.id === 'triposr' && jobTimingProfile?.sampleCount}
          <span>Running beyond the learned median · {jobTimingProfile.confidence} confidence</span>
        {:else if jobAdapter?.manifest.id === 'triposr'}
          <span>Learning timing from this successful run</span>
        {:else}
          <span>Estimated development timing</span>
        {/if}
      </div>
      {#if jobAdapter?.manifest.id === 'triposr'}
        <div class="estimate-note">{jobTimingProfile?.sampleCount ? `Learned locally from ${jobTimingProfile.sampleCount} comparable run${jobTimingProfile.sampleCount === 1 ? '' : 's'}.` : 'No prior comparable run exists yet; stage progress is shown without a learned ETA.'}</div>
      {/if}
      <ol class="stages">
        {#each (jobAdapter ?? activeAdapter).manifest.stages as stage}
          <li class={stageState(stage.id)}><span>{stageState(stage.id) === 'done' ? '✓' : stageState(stage.id) === 'active' ? '●' : '○'}</span>{stage.label}</li>
        {/each}
      </ol>
      <button type="button" class="secondary danger" on:click={() => controller?.abort()}>Cancel</button>
    </section>
  {/if}

  {#if result}
    <section class="result-section">
      <div class="result-heading">
        <div><span class="eyebrow">GENERATED LOCALLY</span><h2>Preview</h2></div>
        <div class="result-stats"><span>{result.elapsedSeconds.toFixed(1)} s</span><span>{result.textured ? 'Textured' : 'Vertex colour'}</span><span>{result.triangles.toLocaleString()} triangles</span></div>
      </div>
      <ModelViewer textureUrl={sourceUrl} modelUrl={previewModelUrl} exportFilename={previewFilename} {wireframe} {showGrid} />
      <div class="viewer-options">
        <label class="check"><input type="checkbox" bind:checked={wireframe} /> Wireframe</label>
        <label class="check"><input type="checkbox" bind:checked={showGrid} /> Grid</label>
        <button type="button" class="secondary" on:click={generate}>Regenerate</button>
      </div>
      {#if result.warning}<p class="result-warning">{result.warning}</p>{/if}
      {#if result.modelId === 'triposr'}
        <p class="development-note">M5 treats this validated production GLB as the canonical master. Use Export for the exact GLB, an OBJ + MTL + texture compatibility package, or geometry-only STL; M4 timing continues to learn only from successful local generations.</p>
      {:else}
        <p class="development-note">This is the deterministic Mock3D fallback. Install and select TripoSR in Model Manager to enable production inference, canonical production assets and learned local timing.</p>
      {/if}
    </section>
  {/if}
</main>

<ModelManager
  bind:open={modelManagerOpen}
  bind:preferredModelId={preferredProductionModelId}
  bind:runtimeStates
  {hardware}
  assessments={modelAssessments}
/>

<footer>
  <span>Still2Solid M5</span>
  <span>Local-first · canonical GLB + derived exports · learned ETA stays local · no telemetry</span>
</footer>
