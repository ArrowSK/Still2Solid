<script lang="ts">
  import { onMount } from 'svelte';
  import { modelCandidates } from './modelCatalog';
  import { recommendedProductionModel } from './recommendation';
  import type { HardwareProfile, ModelAssessment } from './types';

  export let open = false;
  export let hardware: HardwareProfile;
  export let assessments: ModelAssessment[] = [];
  export let preferredModelId = '';

  const STORAGE_KEY = 'still2solid.preferredProductionModel';
  const productionCandidates = modelCandidates.filter((candidate) => candidate.manifest.id !== 'mock3d');

  $: automaticRecommendation = recommendedProductionModel(assessments);

  function assessmentFor(modelId: string): ModelAssessment | undefined {
    return assessments.find((assessment) => assessment.modelId === modelId);
  }

  function canPrefer(assessment: ModelAssessment | undefined): boolean {
    return !!assessment && ['recommended', 'compatible', 'slow'].includes(assessment.compatibility);
  }

  function prefer(modelId: string) {
    preferredModelId = modelId;
    localStorage.setItem(STORAGE_KEY, modelId);
  }

  function clearPreference() {
    preferredModelId = '';
    localStorage.removeItem(STORAGE_KEY);
  }

  function statusClass(assessment: ModelAssessment | undefined): string {
    return assessment?.compatibility ?? 'unsupported';
  }

  onMount(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (open && event.key === 'Escape') open = false;
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  });
</script>

{#if open}
  <div class="model-manager-layer">
    <button class="backdrop" aria-label="Close Model Manager" type="button" on:click={() => (open = false)}></button>
    <section class="model-manager" role="dialog" aria-modal="true" aria-labelledby="model-manager-title">
      <header>
        <div>
          <span class="eyebrow">M2 · HARDWARE-AWARE</span>
          <h2 id="model-manager-title">Model Manager</h2>
        </div>
        <button class="secondary close" type="button" on:click={() => (open = false)}>Close</button>
      </header>

      <div class="hardware-card">
        <div>
          <span>Detected system</span>
          <strong>{hardware.chip}</strong>
          <small>{hardware.platform} · {hardware.architecture}{hardware.memoryGb ? ` · ${hardware.memoryGb.toFixed(1)} GB memory` : ''}</small>
        </div>
        <div class="hardware-backend">
          <span>Preferred backend</span>
          <strong>{hardware.preferredBackend}</strong>
          {#if hardware.accelerators.length}
            <small>{hardware.accelerators.map((accelerator) => `${accelerator.name}${accelerator.memoryGb ? ` · ${accelerator.memoryGb.toFixed(1)} GB` : ''}`).join(' · ')}</small>
          {:else}
            <small>No accelerated device detected</small>
          {/if}
        </div>
      </div>

      {#if automaticRecommendation}
        <div class="recommendation-summary">
          <span>Automatic recommendation</span>
          <strong>{modelCandidates.find((candidate) => candidate.manifest.id === automaticRecommendation?.modelId)?.manifest.name}</strong>
          <small>Selected from permissively licensed candidates that clear the current hardware policy.</small>
        </div>
      {:else}
        <div class="recommendation-summary caution">
          <span>Automatic recommendation</span>
          <strong>No production model clears the safe threshold</strong>
          <small>Mock3D remains available. The manager does not hide memory or platform constraints.</small>
        </div>
      {/if}

      <div class="candidate-list">
        {#each productionCandidates as candidate}
          {@const assessment = assessmentFor(candidate.manifest.id)}
          <article class="candidate-card">
            <div class="candidate-heading">
              <div>
                <div class="name-row">
                  <h3>{candidate.manifest.name}</h3>
                  {#if preferredModelId === candidate.manifest.id}<span class="preferred">Preferred</span>{/if}
                </div>
                <p>{candidate.summary}</p>
              </div>
              <span class={`status ${statusClass(assessment)}`}>{assessment?.label ?? 'Unsupported'}</span>
            </div>

            <div class="facts">
              <span><strong>{(candidate.manifest.diskSizeMb / 1024).toFixed(1)} GB</strong> checkpoint</span>
              <span><strong>{candidate.manifest.license}</strong> licence</span>
              <span><strong>{candidate.manifest.supportsPbr ? 'PBR' : candidate.manifest.supportsTexture ? 'Textured' : 'Geometry'}</strong> output</span>
            </div>

            <div class="source">{candidate.sourceLabel}</div>

            {#if assessment}
              <details>
                <summary>Why this status?</summary>
                <div class="details-body">
                  {#each assessment.reasons as reason}<p>{reason}</p>{/each}
                  {#each assessment.caveats as caveat}<p class="caveat">{caveat}</p>{/each}
                  <p class="licence-note">{candidate.licenseNote}</p>
                </div>
              </details>
            {/if}

            <div class="candidate-actions">
              {#if preferredModelId === candidate.manifest.id}
                <button class="secondary" type="button" on:click={clearPreference}>Use automatic choice</button>
              {:else}
                <button class="secondary" type="button" disabled={!canPrefer(assessment)} on:click={() => prefer(candidate.manifest.id)}>
                  Prefer for M3
                </button>
              {/if}
              <span>{candidate.availability === 'gated' ? 'Gated upstream access' : 'Runtime adapter arrives in M3'}</span>
            </div>
          </article>
        {/each}
      </div>

      <footer class="manager-footer">
        <strong>M2 does not download or execute production model weights.</strong>
        <span>It establishes the catalogue, hardware probe, compatibility policy and persistent production-model preference. Pinned downloads, checksums and isolated workers remain M3 work.</span>
      </footer>
    </section>
  </div>
{/if}

<style>
  .model-manager-layer { position: fixed; inset: 0; z-index: 50; display: grid; place-items: center; padding: 24px; }
  .backdrop { position: absolute; inset: 0; width: 100%; height: 100%; border: 0; background: rgba(4, 6, 10, .76); backdrop-filter: blur(8px); }
  .model-manager { position: relative; width: min(920px, 100%); max-height: calc(100vh - 48px); overflow: auto; border: 1px solid var(--border); border-radius: 22px; background: #13161c; box-shadow: 0 28px 90px rgba(0,0,0,.52); }
  header { position: sticky; top: 0; z-index: 2; display: flex; align-items: center; justify-content: space-between; gap: 20px; padding: 20px 22px; border-bottom: 1px solid var(--border); background: rgba(19,22,28,.96); backdrop-filter: blur(10px); }
  h2 { margin: 3px 0 0; font-size: 24px; }
  .close { padding: 8px 12px; }
  .hardware-card { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin: 20px 22px 0; padding: 16px; border: 1px solid var(--border); border-radius: 15px; background: var(--panel); }
  .hardware-card > div { display: grid; gap: 5px; }
  .hardware-card span, .recommendation-summary span, .facts, .source, .candidate-actions span, .hardware-card small { color: var(--muted); font-size: 12px; }
  .hardware-backend { text-align: right; }
  .recommendation-summary { display: grid; gap: 5px; margin: 12px 22px 0; padding: 14px 16px; border: 1px solid #3c4e7e; border-radius: 14px; background: #1a2131; }
  .recommendation-summary strong { color: var(--accent-strong); }
  .recommendation-summary.caution { border-color: #5b4b31; background: #211d16; }
  .recommendation-summary.caution strong { color: #e5c38a; }
  .candidate-list { display: grid; gap: 12px; padding: 18px 22px 22px; }
  .candidate-card { padding: 17px; border: 1px solid var(--border); border-radius: 16px; background: var(--panel); }
  .candidate-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  .candidate-heading h3 { margin: 0; font-size: 17px; }
  .candidate-heading p { max-width: 620px; margin: 6px 0 0; color: var(--muted); font-size: 13px; line-height: 1.45; }
  .name-row { display: flex; align-items: center; gap: 8px; }
  .preferred { border: 1px solid #52669d; border-radius: 999px; padding: 2px 6px; color: var(--accent-strong); font-size: 10px; }
  .status { flex: 0 0 auto; border: 1px solid var(--border); border-radius: 999px; padding: 5px 8px; color: var(--muted); font-size: 11px; }
  .status.recommended { border-color: #5573c9; color: #cbd7ff; background: #202942; }
  .status.compatible { border-color: #496d5c; color: #bfe3d0; background: #18271f; }
  .status.slow, .status.memory-constrained { border-color: #695a39; color: #e5cf9e; background: #292317; }
  .status.unsupported { border-color: #624242; color: #e2aaaa; background: #28191b; }
  .facts { display: flex; flex-wrap: wrap; gap: 12px 18px; margin-top: 14px; }
  .facts strong { color: var(--text); font-weight: 650; }
  .source { margin-top: 10px; }
  details { margin-top: 12px; color: var(--accent-strong); font-size: 12px; }
  summary { cursor: pointer; }
  .details-body { margin-top: 9px; padding: 12px; border: 1px solid var(--border); border-radius: 11px; color: var(--text); background: #11141a; }
  .details-body p { margin: 0 0 7px; line-height: 1.4; }
  .details-body p:last-child { margin-bottom: 0; }
  .details-body .caveat { color: #e1c793; }
  .licence-note { color: var(--muted); }
  .candidate-actions { display: flex; align-items: center; justify-content: space-between; gap: 14px; margin-top: 14px; }
  .candidate-actions button { padding: 8px 11px; }
  .manager-footer { display: grid; gap: 5px; padding: 16px 22px 20px; border-top: 1px solid var(--border); color: var(--muted); font-size: 12px; line-height: 1.45; }
  .manager-footer strong { color: var(--text); }
  @media (max-width: 680px) {
    .model-manager-layer { padding: 10px; }
    .model-manager { max-height: calc(100vh - 20px); }
    .hardware-card { grid-template-columns: 1fr; }
    .hardware-backend { text-align: left; }
    .candidate-heading, .candidate-actions { align-items: flex-start; flex-direction: column; }
  }
</style>
