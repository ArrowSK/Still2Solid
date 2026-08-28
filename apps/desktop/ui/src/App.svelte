<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import ModelViewer from './lib/ModelViewer.svelte';
  import { getHardwareProfile } from './lib/hardware';
  import { Mock3DAdapter } from './lib/mockAdapter';
  import type { GenerationResult, HardwareProfile, ProgressEvent, QualityPreset } from './lib/types';

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
    platform: 'Detecting…', architecture: '—', chip: 'Detecting hardware…', memoryGb: 0, osVersion: '—', preferredBackend: 'Auto',
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

  onMount(async () => {
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

<svelte:head><title>Still2Solid · M1</title></svelte:head>

<header class="topbar">
  <div>
    <div class="eyebrow">LOCAL IMAGE → 3D</div>
    <h1>Still2Solid</h1>
  </div>
  <div class="milestone">M1 · Application shell</div>
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
      <p>or choose one from this Mac. The file stays local.</p>
      <button type="button" class="primary">Choose image…</button>
      <input bind:this={fileInput} class="visually-hidden" type="file" accept="image/jpeg,image/png,image/webp,image/heic,image/heif" on:change={(event) => selectFile(event.currentTarget.files?.[0])} />
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
            <label>Model</label>
            <strong>{adapter.manifest.name} <span class="badge">Development adapter</span></strong>
          </div>
          <details>
            <summary>Why this model?</summary>
            <div class="explanation">
              <p><strong>Mock3D is intentionally selected for M1.</strong> It validates the application shell, adapter contract, progress reporting and preview without downloading or executing external model weights.</p>
              <p>Your detected hardware is <strong>{hardware.chip}</strong>{hardware.memoryGb ? ` with ${hardware.memoryGb.toFixed(1)} GB memory` : ''}. Hardware-based production model recommendation is M2.</p>
              <p>Licence: {adapter.manifest.license}. No external model licence applies.</p>
            </div>
          </details>
        </div>

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
            <div class="diagnostic"><span>Preferred backend</span><strong>{hardware.preferredBackend}</strong></div>
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
      <ModelViewer {sourceUrl} textureUrl={sourceUrl} {wireframe} {showGrid} />
      <div class="viewer-options">
        <label class="check"><input type="checkbox" bind:checked={wireframe} /> Wireframe</label>
        <label class="check"><input type="checkbox" bind:checked={showGrid} /> Grid</label>
        <button type="button" class="secondary" on:click={generate}>Regenerate</button>
      </div>
      <p class="development-note">This is the deterministic M1 Mock3D result. It intentionally uses a known cube-like mesh and the selected image as a texture so the complete desktop workflow can be tested before production model inference is added.</p>
    </section>
  {/if}
</main>

<footer>
  <span>Still2Solid M1</span>
  <span>Local-first · no external model weights · no telemetry</span>
</footer>
