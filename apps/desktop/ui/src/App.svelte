<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import ModelManager from './lib/ModelManager.svelte';
  import ModelViewer from './lib/ModelViewer.svelte';
  import { getHardwareProfile } from './lib/hardware';
  import { modelCandidateById } from './lib/modelCatalog';
  import { Mock3DAdapter } from './lib/mockAdapter';
  import { assessModels, recommendedProductionModel } from './lib/recommendation';
  import type { GenerationResult, HardwareProfile, ModelAssessment, ProgressEvent, QualityPreset } from './lib/types';

  const adapter = new Mock3DAdapter();
  const qualities: Array<{ id: QualityPreset; title: string; detail: string }> = [
    { id: 'fast', title: 'Fast', detail: 'Lower-density development mesh' },
    { id: 'standard', title: 'Standard', detail: 'Balanced development preset' },
    { id: 'best', title: 'Best', detail: 'Highest-density development mesh' },
  ];

  let quality: QualityPreset = 'standard';
  let sourceFile: File | null = null;
  let sourceUrl = '';
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
  let advanced = false;
  let backend: 'auto' | 'metal' | 'cpu' = 'auto';
  let backgroundRemoval = true;
  let wireframe = false;
  let showGrid = true;
  let fileInput: HTMLInputElement;
  let modelManagerOpen = false;
  let preferredProductionModelId = '';
  let modelAssessments: ModelAssessment[] = [];

  $: modelAssessments = assessModels(hardware);
  $: automaticRecommendation = recommendedProductionModel(modelAssessments);
  $: preferredAssessment = modelAssessments.find((item) => item.modelId === preferredProductionModelId);
  $: preferredIsUsable = preferredAssessment && ['recommended', 'compatible', 'slow'].includes(preferredAssessment.compatibility);
  $: productionAssessment = preferredIsUsable ? preferredAssessment : automaticRecommendation;
  $: productionCandidate = productionAssessment ? modelCandidateById(productionAssessment.modelId) : undefined;

  onMount(async () => {
    preferredProductionModelId = localStorage.getItem('still2solid.preferredProductionModel') ?? '';
    hardware = await getHardwareProfile();
  });

  onDestroy(() => {
    controller?.abort();
    if (sourceUrl) URL.revokeObjectURL(sourceUrl);
  });

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
    result = null;
    progress = null;
  }

  function onDrop(event: DragEvent) {
    event.preventDefault();
    selectFile(event.dataTransfer?.files?.[0]);
  }

  async function generate() {
    if (!sourceFile || generating) return;
    error = '';
    result = null;
    progress = null;
    generating = true;
    controller = new AbortController();

    try {
      result = await adapter.generate(
        {
          quality,
          sourceName: sourceFile.name,
          sourceSizeBytes: sourceFile.size,
          backend,
          backgroundRemoval,
        },
        (event) => (progress = event),
        controller.signal,
      );
    } catch (caught) {
      if (caught instanceof DOMException && caught.name === 'AbortError') error = 'Generation cancelled.';
      else error = caught instanceof Error ? caught.message : 'Generation failed.';
    } finally {
      generating = false;
      controller = null;
    }
  }

  function reset() {
    controller?.abort();
    sourceFile = null;
    result = null;
    progress = null;
    error = '';
    if (sourceUrl) URL.revokeObjectURL(sourceUrl);
    sourceUrl = '';
  }

  const stageState = (id: string) => {
    if (!progress) return 'pending';
    const activeIndex = adapter.manifest.stages.findIndex((stage) => stage.id === progress?.stageId);
    const index = adapter.manifest.stages.findIndex((stage) => stage.id === id);
    if (index < activeIndex) return 'done';
    if (index === activeIndex) return progress.stageProgress >= 1 ? 'done' : 'active';
    return 'pending';
  };
</script>

<svelte:head><title>Still2Solid · M2</title></svelte:head>

<header class="topbar">
  <div>
    <div class="eyebrow">LOCAL IMAGE → 3D</div>
    <h1>Still2Solid</h1>
  </div>
  <div class="top-actions">
    <button type="button" class="secondary model-manager-button" on:click={() => (modelManagerOpen = true)}>Models</button>
    <div class="milestone">M2 · Model intelligence</div>
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
        {#if productionCandidate}
          <span>{preferredIsUsable ? 'Preferred production model' : 'Recommended for this computer'}</span>
          <strong>{productionCandidate.manifest.name}</strong>
          <small>{productionAssessment?.label} · inspect in Model Manager</small>
        {:else}
          <span>Production model</span>
          <strong>No safe automatic recommendation yet</strong>
          <small>Open Model Manager for the hardware assessment</small>
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
            <label>Active adapter</label>
            <strong>{adapter.manifest.name} <span class="badge">Development adapter</span></strong>
          </div>
          <details>
            <summary>Why this model?</summary>
            <div class="explanation">
              <p><strong>Mock3D remains the active inference adapter in M2.</strong> M2 adds model discovery and hardware-aware recommendation without pretending that a production worker already exists.</p>
              {#if productionCandidate && productionAssessment}
                <p><strong>{productionCandidate.manifest.name}</strong> is currently {preferredIsUsable ? 'your preferred M3 candidate' : 'the automatic production recommendation'} for this hardware. Status: {productionAssessment.label}.</p>
                <p>{productionAssessment.reasons[0]}</p>
              {:else}
                <p>No production candidate currently clears the safe automatic threshold on the detected hardware.</p>
              {/if}
              <p>Open Model Manager for the full hardware and licence rationale.</p>
            </div>
          </details>
        </div>

        <button type="button" class="recommendation-strip" on:click={() => (modelManagerOpen = true)}>
          <span>{preferredIsUsable ? 'Preferred for M3' : 'Production recommendation'}</span>
          <strong>{productionCandidate?.manifest.name ?? 'No safe recommendation'}</strong>
          <small>{productionAssessment?.label ?? 'Inspect hardware constraints'}</small>
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
                <option value="cpu">CPU</option>
              </select>
            </label>
            <label class="check"><input type="checkbox" bind:checked={backgroundRemoval} disabled={generating} /> Isolate foreground object</label>
            <div class="diagnostic"><span>Detected</span><strong>{hardware.platform} · {hardware.architecture}</strong></div>
            <div class="diagnostic"><span>Memory</span><strong>{hardware.memoryGb ? `${hardware.memoryGb.toFixed(1)} GB` : 'Unavailable in browser preview'}</strong></div>
            <div class="diagnostic"><span>Preferred backend</span><strong>{hardware.preferredBackend}</strong></div>
            <div class="diagnostic"><span>Accelerator</span><strong>{hardware.accelerators[0]?.name ?? 'None detected'}</strong></div>
          </div>
        {/if}

        <button type="button" class="primary generate" on:click={generate} disabled={generating || !sourceFile}>
          {generating ? 'Generating…' : 'Generate 3D'}
        </button>
      </div>
    </section>
  {/if}

  {#if error}
    <div class="message" role="status">{error}</div>
  {/if}

  {#if generating && progress}
    <section class="progress-card" aria-live="polite">
      <div class="progress-heading">
        <div><span class="eyebrow">MOCK GENERATION</span><h2>{progress.stageName}</h2></div>
        <strong>{Math.round(progress.overallProgress * 100)}%</strong>
      </div>
      <div class="bar"><div style={`width:${progress.overallProgress * 100}%`}></div></div>
      <div class="eta"><span>Estimated progress</span><span>About {Math.ceil(progress.etaSeconds)} s remaining</span></div>
      <ol class="stages">
        {#each adapter.manifest.stages as stage}
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
        <div class="result-stats"><span>{result.elapsedSeconds.toFixed(1)} s</span><span>Textured</span><span>{result.triangles.toLocaleString()} triangles</span></div>
      </div>
      <ModelViewer textureUrl={sourceUrl} {wireframe} {showGrid} />
      <div class="viewer-options">
        <label class="check"><input type="checkbox" bind:checked={wireframe} /> Wireframe</label>
        <label class="check"><input type="checkbox" bind:checked={showGrid} /> Grid</label>
        <button type="button" class="secondary" on:click={generate}>Regenerate</button>
      </div>
      <p class="development-note">This is still the deterministic Mock3D result. M2 selects and explains production candidates, but production model downloads and isolated inference workers are intentionally deferred to M3.</p>
    </section>
  {/if}
</main>

<ModelManager
  bind:open={modelManagerOpen}
  bind:preferredModelId={preferredProductionModelId}
  {hardware}
  assessments={modelAssessments}
/>

<footer>
  <span>Still2Solid M2</span>
  <span>Local-first · hardware-aware model catalogue · no telemetry</span>
</footer>
