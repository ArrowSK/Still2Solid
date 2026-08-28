<script lang="ts">
  import { onMount } from 'svelte';
  import { modelCandidates } from './modelCatalog';
  import { recommendedProductionModel } from './recommendation';
  import {
    getModelRuntimeStates,
    installModel,
    installSf3d,
    listenForInstallProgress,
    uninstallModel,
    uninstallSf3d,
  } from './runtime';
  import type {
    HardwareProfile,
    ModelAssessment,
    ModelInstallProgress,
    ModelRuntimeState,
  } from './types';

  export let open = false;
  export let hardware: HardwareProfile;
  export let assessments: ModelAssessment[] = [];
  export let preferredModelId = '';
  export let runtimeStates: ModelRuntimeState[] = [];

  const STORAGE_KEY = 'still2solid.preferredProductionModel';
  const productionCandidates = modelCandidates.filter((candidate) => candidate.manifest.id !== 'mock3d');

  let installProgress: ModelInstallProgress | null = null;
  let busyModelId = '';
  let managerError = '';
  let sf3dToken = '';
  let sf3dLicenseAccepted = false;

  $: automaticRecommendation = recommendedProductionModel(assessments);

  function assessmentFor(modelId: string): ModelAssessment | undefined {
    return assessments.find((assessment) => assessment.modelId === modelId);
  }

  function runtimeFor(modelId: string): ModelRuntimeState | undefined {
    return runtimeStates.find((runtime) => runtime.modelId === modelId);
  }

  function canInstall(assessment: ModelAssessment | undefined): boolean {
    return !!assessment && ['recommended', 'compatible', 'slow', 'memory-constrained'].includes(assessment.compatibility);
  }

  function canUse(assessment: ModelAssessment | undefined, runtime: ModelRuntimeState | undefined): boolean {
    return !!assessment && !!runtime?.canGenerate && assessment.compatibility !== 'unsupported' && assessment.compatibility !== 'license-restricted';
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

  function replaceRuntime(next: ModelRuntimeState) {
    runtimeStates = [...runtimeStates.filter((runtime) => runtime.modelId !== next.modelId), next];
  }

  function formatBytes(bytes: number): string {
    if (!bytes) return '—';
    return bytes >= 1024 ** 3 ? `${(bytes / 1024 ** 3).toFixed(1)} GB` : `${(bytes / 1024 ** 2).toFixed(0)} MB`;
  }

  async function install(candidateId: string, experimental: boolean) {
    managerError = '';
    busyModelId = candidateId;
    installProgress = null;
    try {
      const state = candidateId === 'sf3d'
        ? await installSf3d(sf3dToken, sf3dLicenseAccepted)
        : await installModel(candidateId);
      replaceRuntime(state);
      if (candidateId === 'sf3d') sf3dToken = '';
      if (state.canGenerate && (experimental || preferredModelId === candidateId)) prefer(candidateId);
    } catch (caught) {
      managerError = caught instanceof Error ? caught.message : String(caught);
      runtimeStates = await getModelRuntimeStates();
    } finally {
      busyModelId = '';
      installProgress = null;
    }
  }

  async function uninstall(candidateId: string) {
    managerError = '';
    busyModelId = candidateId;
    try {
      const state = candidateId === 'sf3d' ? await uninstallSf3d() : await uninstallModel(candidateId);
      replaceRuntime(state);
      if (preferredModelId === candidateId) clearPreference();
    } catch (caught) {
      managerError = caught instanceof Error ? caught.message : String(caught);
    } finally {
      busyModelId = '';
    }
  }

  onMount(() => {
    let stopInstallListener: (() => void) | undefined;
    void getModelRuntimeStates().then((states) => (runtimeStates = states));
    void listenForInstallProgress((progress) => {
      installProgress = progress;
      busyModelId = progress.modelId;
    }).then((unlisten) => (stopInstallListener = unlisten));

    const onKeyDown = (event: KeyboardEvent) => {
      if (open && event.key === 'Escape' && !busyModelId) open = false;
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      stopInstallListener?.();
      window.removeEventListener('keydown', onKeyDown);
    };
  });
</script>

{#if open}
  <div class="model-manager-layer">
    <button class="backdrop" aria-label="Close Model Manager" type="button" disabled={!!busyModelId} on:click={() => (open = false)}></button>
    <section class="model-manager" role="dialog" aria-modal="true" aria-labelledby="model-manager-title">
      <header>
        <div>
          <span class="eyebrow">M8 · LOCAL MODEL RUNTIMES</span>
          <h2 id="model-manager-title">Model Manager</h2>
        </div>
        <button class="secondary close" type="button" disabled={!!busyModelId} on:click={() => (open = false)}>Close</button>
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
          <small>{hardware.accelerators.length ? hardware.accelerators.map((accelerator) => `${accelerator.name}${accelerator.memoryGb ? ` · ${accelerator.memoryGb.toFixed(1)} GB` : ''}`).join(' · ') : 'No accelerated device detected'}</small>
        </div>
      </div>

      {#if automaticRecommendation}
        <div class="recommendation-summary">
          <span>Automatic recommendation</span>
          <strong>{modelCandidates.find((candidate) => candidate.manifest.id === automaticRecommendation?.modelId)?.manifest.name}</strong>
          <small>Automatic use still requires an installed, verified runtime. Conditional/gated models are never auto-selected.</small>
        </div>
      {:else}
        <div class="recommendation-summary caution">
          <span>Automatic recommendation</span>
          <strong>No production model clears the safe threshold</strong>
          <small>Explicit experimental installation remains possible where the compatibility policy allows it.</small>
        </div>
      {/if}

      {#if managerError}<div class="manager-error" role="status">{managerError}</div>{/if}

      <div class="candidate-list">
        {#each productionCandidates as candidate}
          {@const assessment = assessmentFor(candidate.manifest.id)}
          {@const runtime = runtimeFor(candidate.manifest.id)}
          {@const isBusy = busyModelId === candidate.manifest.id}
          <article class="candidate-card">
            <div class="candidate-heading">
              <div>
                <div class="name-row">
                  <h3>{candidate.manifest.name}</h3>
                  {#if preferredModelId === candidate.manifest.id}<span class="preferred">In use</span>{/if}
                  {#if runtime?.verified}<span class="verified">Verified install</span>{/if}
                  {#if candidate.availability === 'gated'}<span class="gated">Gated</span>{/if}
                </div>
                <p>{candidate.summary}</p>
              </div>
              <span class={`status ${statusClass(assessment)}`}>{assessment?.label ?? 'Unsupported'}</span>
            </div>

            <div class="facts">
              <span><strong>{(candidate.manifest.diskSizeMb / 1024).toFixed(1)} GB</strong> checkpoint</span>
              <span><strong>{candidate.manifest.license}</strong> licence</span>
              <span><strong>{candidate.manifest.supportsPbr ? 'PBR' : candidate.manifest.supportsTexture ? 'Textured' : 'Geometry'}</strong> output</span>
              {#if runtime?.installed}<span><strong>{formatBytes(runtime.installedBytes)}</strong> installed</span>{/if}
            </div>

            <div class="source">{candidate.sourceLabel}</div>

            {#if assessment}
              <details>
                <summary>Why this status?</summary>
                <div class="details-body">
                  {#each assessment.reasons as reason}<p>{reason}</p>{/each}
                  {#each assessment.caveats as caveat}<p class="caveat">{caveat}</p>{/each}
                  <p class="licence-note">{candidate.licenseNote}</p>
                  {#if runtime}
                    <p class="runtime-note"><strong>Runtime:</strong> {runtime.detail}</p>
                    {#if runtime.pythonVersion}<p>Python {runtime.pythonVersion} · source {runtime.sourceRevision.slice(0, 8)} · checkpoint SHA-256 {runtime.weightSha256.slice(0, 12)}…</p>{/if}
                  {/if}
                </div>
              </details>
            {/if}

            {#if candidate.manifest.id === 'sf3d' && !runtime?.canGenerate}
              <div class="gated-install">
                <strong>Stable Fast 3D access</strong>
                <p>Installation requires access to the gated Hugging Face model and explicit acceptance of the Stability AI Community License. The token is passed only to the installer process and is not written to Still2Solid's manifest or settings.</p>
                <label>Hugging Face read token
                  <input type="password" bind:value={sf3dToken} autocomplete="off" spellcheck="false" placeholder="hf_…" disabled={isBusy} />
                </label>
                <label class="accept"><input type="checkbox" bind:checked={sf3dLicenseAccepted} disabled={isBusy} /> I have reviewed and accept the model licence terms for my use.</label>
              </div>
            {/if}

            {#if isBusy && installProgress?.modelId === candidate.manifest.id}
              <div class="install-progress" aria-live="polite">
                <div><span>{installProgress.message}</span><strong>{Math.round(installProgress.overallProgress * 100)}%</strong></div>
                <div class="install-bar"><div style={`width:${installProgress.overallProgress * 100}%`}></div></div>
                {#if installProgress.bytesDownloaded && installProgress.bytesTotal}<small>{formatBytes(installProgress.bytesDownloaded)} / {formatBytes(installProgress.bytesTotal)}</small>{/if}
              </div>
            {/if}

            <div class="candidate-actions">
              <div class="button-row">
                {#if candidate.runtimeAdapter}
                  {#if runtime?.canGenerate}
                    {#if preferredModelId === candidate.manifest.id}
                      <button class="secondary" type="button" disabled={isBusy} on:click={clearPreference}>Use automatic choice</button>
                    {:else}
                      <button class="secondary" type="button" disabled={!canUse(assessment, runtime) || isBusy} on:click={() => prefer(candidate.manifest.id)}>Use for generation</button>
                    {/if}
                    <button class="secondary danger" type="button" disabled={isBusy} on:click={() => uninstall(candidate.manifest.id)}>Uninstall</button>
                  {:else if runtime?.status === 'unavailable'}
                    <button class="secondary" type="button" disabled>Desktop build required</button>
                  {:else}
                    <button
                      class="secondary"
                      type="button"
                      disabled={!canInstall(assessment) || isBusy || (candidate.manifest.id === 'sf3d' && (!sf3dLicenseAccepted || sf3dToken.trim().length < 8))}
                      on:click={() => install(candidate.manifest.id, assessment?.compatibility === 'memory-constrained')}
                    >{runtime?.status === 'broken' ? 'Reinstall' : assessment?.compatibility === 'memory-constrained' ? 'Install experimental' : 'Install'}</button>
                  {/if}
                {:else}
                  <button class="secondary" type="button" disabled>No audited adapter</button>
                {/if}
              </div>
              <span>{candidate.runtimeAdapter ? 'One-shot isolated worker · unloads after every generation' : 'Catalogue only'}</span>
            </div>
          </article>
        {/each}
      </div>

      <footer class="manager-footer">
        <strong>M8 includes two audited production adapters: TripoSR and gated Stable Fast 3D.</strong>
        <span>Release builds use Still2Solid's pinned bundled Python 3.12 runtime. Model source/checkpoints are pinned and verified before activation; generation is local and one-shot with no localhost inference server.</span>
        <span>TripoSR remains the automatic permissive default where safe. Stable Fast 3D is always an explicit opt-in because its licence is conditional and its MPS path is experimental.</span>
      </footer>
    </section>
  </div>
{/if}

<style>
  .model-manager-layer { position: fixed; inset: 0; z-index: 50; display: grid; place-items: center; padding: 24px; }
  .backdrop { position: absolute; inset: 0; width: 100%; height: 100%; border: 0; background: rgba(4,6,10,.76); backdrop-filter: blur(8px); }
  .model-manager { position: relative; width: min(940px,100%); max-height: calc(100vh - 48px); overflow: auto; border: 1px solid var(--border); border-radius: 22px; background: #13161c; box-shadow: 0 28px 90px rgba(0,0,0,.52); }
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
  .manager-error { margin: 12px 22px 0; padding: 12px 14px; border: 1px solid #68454a; border-radius: 12px; color: #ffc2c7; background: #281a1d; font-size: 12px; line-height: 1.45; white-space: pre-wrap; }
  .candidate-list { display: grid; gap: 12px; padding: 18px 22px 22px; }
  .candidate-card { padding: 17px; border: 1px solid var(--border); border-radius: 16px; background: var(--panel); }
  .candidate-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  .candidate-heading h3 { margin: 0; font-size: 17px; }
  .candidate-heading p { max-width: 620px; margin: 6px 0 0; color: var(--muted); font-size: 13px; line-height: 1.45; }
  .name-row { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; }
  .preferred, .verified, .gated { border: 1px solid #52669d; border-radius: 999px; padding: 2px 6px; color: var(--accent-strong); font-size: 10px; }
  .verified { border-color: #466b59; color: #b8dfca; }
  .gated { border-color: #695a39; color: #e5cf9e; }
  .status { flex: 0 0 auto; border: 1px solid var(--border); border-radius: 999px; padding: 5px 8px; color: var(--muted); font-size: 11px; }
  .status.recommended { border-color: #5573c9; color: #cbd7ff; background: #202942; }
  .status.compatible { border-color: #496d5c; color: #bfe3d0; background: #18271f; }
  .status.slow, .status.memory-constrained { border-color: #695a39; color: #e5cf9e; background: #292317; }
  .status.unsupported, .status.license-restricted { border-color: #624242; color: #e2aaaa; background: #28191b; }
  .facts { display: flex; flex-wrap: wrap; gap: 8px 18px; margin-top: 14px; }
  .facts strong { color: var(--text); }
  .source { margin-top: 8px; }
  details { margin-top: 12px; }
  summary { cursor: pointer; color: var(--accent-strong); font-size: 12px; }
  .details-body { margin-top: 8px; color: var(--muted); font-size: 12px; line-height: 1.5; }
  .details-body p { margin: 5px 0; }
  .caveat, .licence-note { color: #e5cf9e; }
  .runtime-note { color: var(--text); }
  .gated-install { display: grid; gap: 8px; margin-top: 14px; padding: 13px; border: 1px solid #695a39; border-radius: 12px; background: #211d16; }
  .gated-install p { margin: 0; color: var(--muted); font-size: 12px; line-height: 1.45; }
  .gated-install label { display: grid; gap: 6px; font-size: 12px; }
  .gated-install input[type='password'] { width: 100%; box-sizing: border-box; border: 1px solid var(--border); border-radius: 9px; padding: 9px 10px; background: #0e1117; color: var(--text); }
  .gated-install .accept { display: flex; align-items: flex-start; gap: 8px; }
  .install-progress { margin-top: 14px; }
  .install-progress > div:first-child { display: flex; justify-content: space-between; gap: 12px; font-size: 12px; }
  .install-bar { height: 6px; overflow: hidden; margin-top: 7px; border-radius: 999px; background: #20242d; }
  .install-bar div { height: 100%; background: var(--accent); }
  .candidate-actions { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-top: 14px; }
  .button-row { display: flex; flex-wrap: wrap; gap: 8px; }
  .manager-footer { display: grid; gap: 6px; padding: 16px 22px 22px; border-top: 1px solid var(--border); color: var(--muted); font-size: 12px; line-height: 1.5; }
  .manager-footer strong { color: var(--text); }
  @media (max-width: 700px) { .model-manager-layer { padding: 8px; } .model-manager { max-height: calc(100vh - 16px); } .hardware-card { grid-template-columns: 1fr; } .hardware-backend { text-align: left; } .candidate-heading, .candidate-actions { align-items: stretch; flex-direction: column; } }
</style>
