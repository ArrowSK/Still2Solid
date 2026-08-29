<script lang="ts">
  import { modelCandidates } from './modelCatalog';
  import { recommendedProductionModel } from './recommendation';
  import type { HardwareProfile, ModelAssessment, ModelRuntimeState } from './types';

  export let open = false;
  export let hardware: HardwareProfile;
  export let assessments: ModelAssessment[] = [];
  export let runtimeStates: ModelRuntimeState[] = [];
  export let preferredModelId = '';
  export let modelManagerOpen = false;

  const COMPLETE_KEY = 'still2solid.firstRunComplete';
  const PREFERENCE_KEY = 'still2solid.preferredProductionModel';
  const productionCandidates = modelCandidates.filter((candidate) => candidate.manifest.id !== 'mock3d');

  let step = 0;
  let choice = '';
  let observedOpen = false;

  $: automaticRecommendation = recommendedProductionModel(assessments);
  $: if (open && !observedOpen) {
    observedOpen = true;
    step = 0;
    choice = preferredModelId || automaticRecommendation?.modelId || bestFallbackChoice();
  }
  $: if (!open && observedOpen) observedOpen = false;

  function assessmentFor(modelId: string): ModelAssessment | undefined {
    return assessments.find((assessment) => assessment.modelId === modelId);
  }

  function runtimeFor(modelId: string): ModelRuntimeState | undefined {
    return runtimeStates.find((runtime) => runtime.modelId === modelId);
  }

  function selectable(modelId: string): boolean {
    const assessment = assessmentFor(modelId);
    return !!assessment && !['unsupported', 'license-restricted'].includes(assessment.compatibility);
  }

  function bestFallbackChoice(): string {
    if (selectable('triposr')) return 'triposr';
    if (selectable('sf3d')) return 'sf3d';
    return 'mock3d';
  }

  function choiceLabel(): string {
    if (choice === 'mock3d') return 'Mock3D for now';
    return productionCandidates.find((candidate) => candidate.manifest.id === choice)?.manifest.name ?? 'No model selected';
  }

  function finish() {
    if (choice && choice !== 'mock3d') {
      preferredModelId = choice;
      localStorage.setItem(PREFERENCE_KEY, choice);
    } else {
      preferredModelId = '';
      localStorage.removeItem(PREFERENCE_KEY);
    }
    localStorage.setItem(COMPLETE_KEY, '1');
    open = false;

    if (choice !== 'mock3d' && !runtimeFor(choice)?.canGenerate) {
      modelManagerOpen = true;
    }
  }

  function decideLater() {
    preferredModelId = '';
    localStorage.removeItem(PREFERENCE_KEY);
    localStorage.setItem(COMPLETE_KEY, '1');
    open = false;
  }
</script>

{#if open}
  <div class="wizard-layer">
    <div class="backdrop"></div>
    <section class="wizard" role="dialog" aria-modal="true" aria-labelledby="wizard-title">
      <div class="steps" aria-label="Setup progress">
        <span class:active={step >= 0}></span>
        <span class:active={step >= 1}></span>
        <span class:active={step >= 2}></span>
      </div>

      {#if step === 0}
        <div class="wizard-body">
          <span class="eyebrow">FIRST START</span>
          <h2 id="wizard-title">Choose the right local 3D model</h2>
          <p>Still2Solid keeps model runtimes separate from the app. This short setup checks this Mac and helps choose what should be installed. You can change the choice later in Models.</p>

          <div class="hardware">
            <div><span>Computer</span><strong>{hardware.chip}</strong></div>
            <div><span>Memory</span><strong>{hardware.memoryGb ? `${hardware.memoryGb.toFixed(1)} GB` : 'Not detected'}</strong></div>
            <div><span>Backend</span><strong>{hardware.preferredBackend}</strong></div>
          </div>

          {#if automaticRecommendation}
            <div class="recommendation">
              <span>Automatic recommendation</span>
              <strong>{productionCandidates.find((candidate) => candidate.manifest.id === automaticRecommendation?.modelId)?.manifest.name}</strong>
              <small>{automaticRecommendation.label}</small>
            </div>
          {:else}
            <div class="recommendation caution">
              <span>Hardware note</span>
              <strong>No production model clears the conservative safe threshold</strong>
              <small>You can still choose an allowed experimental model. Still2Solid will not pretend that the hardware has been validated.</small>
            </div>
          {/if}
        </div>
        <div class="wizard-actions">
          <button type="button" class="text" on:click={decideLater}>Decide later</button>
          <button type="button" class="primary" on:click={() => (step = 1)}>Choose model</button>
        </div>
      {:else if step === 1}
        <div class="wizard-body">
          <span class="eyebrow">MODEL CHOICE</span>
          <h2 id="wizard-title">What should Still2Solid use?</h2>
          <p>The compatibility label comes from this computer. Gated or unsupported choices remain blocked; memory-constrained choices are clearly marked experimental.</p>

          <div class="choices">
            {#each productionCandidates as candidate}
              {@const assessment = assessmentFor(candidate.manifest.id)}
              {@const runtime = runtimeFor(candidate.manifest.id)}
              <button
                type="button"
                class:chosen={choice === candidate.manifest.id}
                class="choice"
                disabled={!selectable(candidate.manifest.id)}
                on:click={() => (choice = candidate.manifest.id)}
              >
                <div>
                  <strong>{candidate.manifest.name}</strong>
                  <span>{candidate.summary}</span>
                </div>
                <div class="choice-status">
                  <b>{assessment?.label ?? 'Unsupported'}</b>
                  <small>{runtime?.canGenerate ? 'Already installed' : candidate.manifest.id === 'sf3d' ? 'Install requires licence acceptance + token' : 'Will install in Model Manager'}</small>
                </div>
              </button>
            {/each}

            <button type="button" class:chosen={choice === 'mock3d'} class="choice" on:click={() => (choice = 'mock3d')}>
              <div>
                <strong>Mock3D for now</strong>
                <span>Use the lightweight development fallback and install a production model later.</span>
              </div>
              <div class="choice-status"><b>Safe fallback</b><small>No model download</small></div>
            </button>
          </div>
        </div>
        <div class="wizard-actions">
          <button type="button" class="text" on:click={() => (step = 0)}>Back</button>
          <button type="button" class="primary" disabled={!choice} on:click={() => (step = 2)}>Continue</button>
        </div>
      {:else}
        <div class="wizard-body">
          <span class="eyebrow">READY</span>
          <h2 id="wizard-title">Use {choiceLabel()}</h2>
          {#if choice === 'mock3d'}
            <p>Still2Solid will start with Mock3D. Production model installation remains available from Models at any time.</p>
          {:else if runtimeFor(choice)?.canGenerate}
            <p>This model is already installed and verified. It will become the preferred production model immediately.</p>
          {:else}
            <p>Your choice will be saved, then Model Manager will open so the verified runtime can be installed. If an automatic TripoSR download fails, recovery options appear only after that failure.</p>
          {/if}

          <div class="summary">
            <span>Selected</span>
            <strong>{choiceLabel()}</strong>
            {#if choice !== 'mock3d'}
              <small>{assessmentFor(choice)?.label}</small>
            {/if}
          </div>
        </div>
        <div class="wizard-actions">
          <button type="button" class="text" on:click={() => (step = 1)}>Back</button>
          <button type="button" class="primary" on:click={finish}>Finish setup</button>
        </div>
      {/if}
    </section>
  </div>
{/if}

<style>
  .wizard-layer { position: fixed; inset: 0; z-index: 70; display: grid; place-items: center; padding: 24px; }
  .backdrop { position: absolute; inset: 0; background: rgba(4,6,10,.82); backdrop-filter: blur(10px); }
  .wizard { position: relative; width: min(760px,100%); max-height: calc(100vh - 48px); overflow: auto; border: 1px solid var(--border); border-radius: 24px; background: #13161c; box-shadow: 0 30px 100px rgba(0,0,0,.58); }
  .steps { display: grid; grid-template-columns: repeat(3,1fr); gap: 8px; padding: 18px 22px 0; }
  .steps span { height: 4px; border-radius: 999px; background: #2a2f38; }
  .steps span.active { background: var(--accent); }
  .wizard-body { display: grid; gap: 15px; padding: 26px 28px 22px; }
  .eyebrow { color: var(--muted); font-size: 11px; letter-spacing: .12em; font-weight: 700; }
  h2 { margin: -4px 0 0; font-size: 28px; }
  p { margin: 0; color: var(--muted); line-height: 1.6; font-size: 14px; }
  .hardware { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: 10px; }
  .hardware div, .summary { display: grid; gap: 5px; padding: 13px; border: 1px solid var(--border); border-radius: 13px; background: var(--panel); }
  .hardware span, .summary span, .summary small { color: var(--muted); font-size: 11px; }
  .hardware strong { font-size: 13px; line-height: 1.35; }
  .recommendation { display: grid; gap: 5px; padding: 14px 16px; border: 1px solid #3c4e7e; border-radius: 14px; background: #1a2131; }
  .recommendation span, .recommendation small { color: var(--muted); font-size: 12px; }
  .recommendation strong { color: var(--accent-strong); }
  .recommendation.caution { border-color: #5b4b31; background: #211d16; }
  .recommendation.caution strong { color: #e5c38a; }
  .choices { display: grid; gap: 10px; }
  .choice { display: grid; grid-template-columns: 1fr auto; gap: 18px; width: 100%; padding: 15px; text-align: left; border: 1px solid var(--border); border-radius: 14px; background: var(--panel); color: var(--text); cursor: pointer; }
  .choice:hover:not(:disabled), .choice.chosen { border-color: #4d8dff; background: #182338; }
  .choice:disabled { opacity: .45; cursor: not-allowed; }
  .choice > div:first-child { display: grid; gap: 5px; }
  .choice span, .choice small { color: var(--muted); font-size: 12px; line-height: 1.45; }
  .choice-status { display: grid; gap: 5px; max-width: 220px; text-align: right; }
  .choice-status b { font-size: 12px; }
  .summary strong { font-size: 20px; }
  .wizard-actions { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 16px 28px 24px; border-top: 1px solid var(--border); }
  button { border-radius: 10px; padding: 10px 14px; font: inherit; font-size: 12px; font-weight: 700; cursor: pointer; }
  button:disabled { opacity: .45; cursor: not-allowed; }
  .primary { border: 1px solid #4d8dff; background: #2f6fe4; color: white; }
  .text { border: 0; background: transparent; color: var(--muted); }
  @media (max-width: 680px) {
    .wizard-layer { padding: 10px; }
    .wizard { max-height: calc(100vh - 20px); }
    .hardware { grid-template-columns: 1fr; }
    .choice { grid-template-columns: 1fr; }
    .choice-status { text-align: left; max-width: none; }
  }
</style>
