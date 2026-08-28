<script lang="ts">
  import { analyseImageBackground, type BackgroundAssessment } from './backgroundAnalysis';

  export let imageUrl = '';
  export let enabled = true;
  export let disabled = false;

  let assessment: BackgroundAssessment | null = null;
  let checking = false;
  let analysisError = '';
  let observedUrl = '';
  let requestId = 0;
  let userChangedSetting = false;

  async function inspect(url: string) {
    const id = ++requestId;
    checking = true;
    assessment = null;
    analysisError = '';
    userChangedSetting = false;

    try {
      const next = await analyseImageBackground(url);
      if (id !== requestId || url !== imageUrl) return;
      assessment = next;
      if (!userChangedSetting) enabled = next.suggestRemoval;
    } catch (caught) {
      if (id !== requestId || url !== imageUrl) return;
      analysisError = caught instanceof Error ? caught.message : 'Background check could not run.';
    } finally {
      if (id === requestId) checking = false;
    }
  }

  function changeSetting(event: Event) {
    userChangedSetting = true;
    enabled = (event.currentTarget as HTMLInputElement).checked;
  }

  $: if (imageUrl && imageUrl !== observedUrl) {
    observedUrl = imageUrl;
    void inspect(imageUrl);
  }
</script>

<section class="background-advisor" class:recommended={assessment?.suggestRemoval} aria-live="polite">
  <div class="background-copy">
    <span class="background-kicker">BACKGROUND CHECK · LOCAL</span>
    {#if checking}
      <strong>Checking the image…</strong>
      <p>Still2Solid is looking only at a tiny local copy of the image edges. Nothing is uploaded.</p>
    {:else if assessment?.kind === 'likely-background'}
      <strong>Background likely detected</strong>
      <p>Foreground isolation is recommended. It usually gives the 3D model a cleaner silhouette and fewer stray surfaces.</p>
      <small>{assessment.reason}</small>
    {:else if assessment?.kind === 'transparent'}
      <strong>The object already looks isolated</strong>
      <p>Transparent pixels reach the image edge, so another removal pass is usually unnecessary.</p>
      <small>{assessment.reason}</small>
    {:else if assessment?.kind === 'uncertain'}
      <strong>Background may be present</strong>
      <p>The image is opaque, but the quick check is not confident. Keep isolation enabled if the object is surrounded by a table, wall, room, sky, floor or other scenery.</p>
      <small>{assessment.reason}</small>
    {:else if analysisError}
      <strong>Background check unavailable</strong>
      <p>You can still choose foreground isolation manually. Generation itself is not blocked.</p>
    {:else}
      <strong>Foreground isolation</strong>
      <p>Remove surrounding scenery before the image reaches the 3D model.</p>
    {/if}
  </div>

  <label class="background-switch">
    <input type="checkbox" checked={enabled} on:change={changeSetting} {disabled} />
    <span>
      <strong>Remove background</strong>
      <small>{enabled ? 'Enabled for this generation' : 'Keep the original image'}</small>
    </span>
  </label>
</section>

<style>
  .background-advisor {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 16px;
    margin-top: 14px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: #141922;
  }

  .background-advisor.recommended {
    border-color: #3b5b79;
    background: linear-gradient(135deg, #14202d, #141922 64%);
  }

  .background-copy { min-width: 0; }
  .background-kicker { display: block; margin-bottom: 5px; color: #7e91ac; font-size: 10px; font-weight: 750; letter-spacing: .1em; }
  .background-copy > strong { display: block; font-size: 13px; }
  .background-copy p { margin: 5px 0 4px; color: var(--muted); font-size: 11px; line-height: 1.45; }
  .background-copy small { color: #6f798a; font-size: 10px; line-height: 1.4; }

  .background-switch {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 156px;
    align-self: center;
    padding: 9px 10px;
    border: 1px solid #344055;
    border-radius: 11px;
    color: var(--text);
    background: #11161e;
    cursor: pointer;
  }
  .background-switch input { width: 17px; height: 17px; accent-color: var(--accent); }
  .background-switch span { display: grid; gap: 2px; }
  .background-switch strong { font-size: 11px; }
  .background-switch small { color: var(--muted); font-size: 9px; }

  @media (max-width: 560px) {
    .background-advisor { grid-template-columns: 1fr; }
    .background-switch { width: 100%; }
  }
</style>
