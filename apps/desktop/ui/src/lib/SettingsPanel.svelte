<script lang="ts">
  import { onMount } from 'svelte';
  import { getModelRuntimeStates, uninstallModel, uninstallSf3d } from './runtime';
  import {
    clearAppCache,
    clearAppOwnedData,
    getStorageSummary,
    openApplicationsFolder,
    type StorageSummary,
  } from './storage';
  import {
    checkForUpdates,
    downloadUpdate,
    listenForUpdateProgress,
    openUpdateInstaller,
    type DownloadedUpdate,
    type UpdateDownloadProgress,
    type UpdateInfo,
  } from './updater';
  import type { ModelRuntimeState } from './types';
  import type { UnlistenFn } from '@tauri-apps/api/event';

  export let open = false;
  export let platform = '';
  export let appVersion = '—';
  export let disabled = false;
  export let runtimeStates: ModelRuntimeState[] = [];
  export let preferredModelId = '';

  let summary: StorageSummary | null = null;
  let busy = false;
  let error = '';
  let message = '';
  let confirmUninstall = false;
  let prepared = false;
  let observedOpen = false;

  let updateInfo: UpdateInfo | null = null;
  let downloadedUpdate: DownloadedUpdate | null = null;
  let updateProgress: UpdateDownloadProgress | null = null;
  let updateBusy = false;
  let updateError = '';
  let updateMessage = '';
  let unlistenUpdateProgress: UnlistenFn | null = null;

  $: if (open && !observedOpen) {
    observedOpen = true;
    void refresh();
  }
  $: if (!open && observedOpen) {
    observedOpen = false;
    confirmUninstall = false;
    prepared = false;
    error = '';
    message = '';
    updateError = '';
    updateMessage = '';
    updateProgress = null;
  }

  function formatBytes(bytes: number): string {
    if (bytes <= 0) return '0 MB';
    if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
    if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(0)} MB`;
    return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  }

  function clearStill2SolidPreferences() {
    for (let index = localStorage.length - 1; index >= 0; index -= 1) {
      const key = localStorage.key(index);
      if (key?.startsWith('still2solid.')) localStorage.removeItem(key);
    }
  }

  async function refresh() {
    summary = await getStorageSummary();
  }

  async function checkUpdate() {
    if (updateBusy) return;
    updateBusy = true;
    updateError = '';
    updateMessage = '';
    updateProgress = null;
    downloadedUpdate = null;
    try {
      updateInfo = await checkForUpdates();
      updateMessage = updateInfo.available
        ? `Still2Solid ${updateInfo.latestVersion} is available.`
        : `Still2Solid ${updateInfo.currentVersion} is up to date.`;
    } catch (caught) {
      updateError = caught instanceof Error ? caught.message : String(caught);
    } finally {
      updateBusy = false;
    }
  }

  async function downloadAndOpenUpdate() {
    if (updateBusy || !updateInfo?.available) return;
    updateBusy = true;
    updateError = '';
    updateMessage = '';
    updateProgress = { downloadedBytes: 0, totalBytes: updateInfo.downloadSize, progress: 0, message: 'Starting update download' };
    try {
      downloadedUpdate = await downloadUpdate();
      await openUpdateInstaller(downloadedUpdate.path);
      updateMessage = `Still2Solid ${downloadedUpdate.version} was verified and the macOS installer was opened. Drag Still2Solid to Applications and choose Replace.`;
    } catch (caught) {
      updateError = caught instanceof Error ? caught.message : String(caught);
    } finally {
      updateBusy = false;
    }
  }

  async function reopenDownloadedUpdate() {
    if (!downloadedUpdate || updateBusy) return;
    updateError = '';
    try {
      await openUpdateInstaller(downloadedUpdate.path);
    } catch (caught) {
      updateError = caught instanceof Error ? caught.message : String(caught);
    }
  }

  async function removeInstalledModels() {
    const installed = runtimeStates.filter((runtime) => runtime.installed);
    for (const runtime of installed) {
      if (runtime.modelId === 'sf3d') await uninstallSf3d();
      else await uninstallModel(runtime.modelId);
    }
    runtimeStates = await getModelRuntimeStates();
    preferredModelId = '';
    localStorage.removeItem('still2solid.preferredProductionModel');
  }

  async function removeModels() {
    if (busy || disabled) return;
    busy = true;
    error = '';
    message = '';
    try {
      await removeInstalledModels();
      await refresh();
      message = 'Downloaded model runtimes were removed. You can reinstall them later from Models.';
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
      runtimeStates = await getModelRuntimeStates();
      await refresh();
    } finally {
      busy = false;
    }
  }

  async function clearCache() {
    if (busy || disabled) return;
    busy = true;
    error = '';
    message = '';
    try {
      summary = await clearAppCache();
      message = 'Temporary Still2Solid files were cleared.';
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
      await refresh();
    } finally {
      busy = false;
    }
  }

  async function prepareForUninstall() {
    if (busy || disabled) return;
    busy = true;
    error = '';
    message = '';
    try {
      await removeInstalledModels();
      summary = await clearAppOwnedData();
      clearStill2SolidPreferences();
      preferredModelId = '';
      runtimeStates = await getModelRuntimeStates();
      summary = await getStorageSummary();
      confirmUninstall = false;
      prepared = true;
      message = 'Cleanup complete. You can now quit Still2Solid and move it from Applications to Trash.';
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
      runtimeStates = await getModelRuntimeStates();
      await refresh();
    } finally {
      busy = false;
    }
  }

  async function openApplications() {
    error = '';
    try {
      await openApplicationsFolder();
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    }
  }

  onMount(() => {
    void listenForUpdateProgress((progress) => {
      updateProgress = progress;
    }).then((unlisten) => {
      unlistenUpdateProgress = unlisten;
    }).catch(() => {
      // Browser-only previews do not expose native updater events.
    });

    const onKeyDown = (event: KeyboardEvent) => {
      if (open && event.key === 'Escape' && !busy && !updateBusy) open = false;
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      unlistenUpdateProgress?.();
    };
  });
</script>

{#if open}
  <div class="settings-layer">
    <button class="backdrop" aria-label="Close Settings" type="button" disabled={busy || updateBusy} on:click={() => (open = false)}></button>
    <section class="settings-panel" role="dialog" aria-modal="true" aria-labelledby="settings-title">
      <header>
        <div>
          <span class="eyebrow">SETTINGS</span>
          <h2 id="settings-title">Still2Solid</h2>
        </div>
        <button class="secondary close" type="button" disabled={busy || updateBusy} on:click={() => (open = false)}>Close</button>
      </header>

      <div class="content">
        <section class="update-card">
          <div class="update-heading">
            <div>
              <span class="eyebrow">SOFTWARE UPDATE</span>
              <h3>Still2Solid {appVersion === '—' ? '' : `v${appVersion}`}</h3>
              <p>Update checks are manual. Still2Solid contacts only this project's public GitHub Releases page when you press Check for updates.</p>
            </div>
            <button class="secondary" type="button" disabled={updateBusy} on:click={checkUpdate}>
              {updateBusy && !updateProgress ? 'Checking…' : 'Check for updates'}
            </button>
          </div>

          {#if updateInfo}
            <div class:available={updateInfo.available} class="update-status">
              <div>
                <span>{updateInfo.available ? 'Update available' : 'Current version'}</span>
                <strong>{updateInfo.available ? `v${updateInfo.latestVersion}` : `v${updateInfo.currentVersion}`}</strong>
                {#if updateInfo.available && updateInfo.downloadSize}<small>{formatBytes(updateInfo.downloadSize)} Apple Silicon DMG</small>{/if}
              </div>
              {#if updateInfo.available}
                <button class="primary" type="button" disabled={updateBusy} on:click={downloadAndOpenUpdate}>
                  {updateBusy ? 'Downloading…' : 'Download & open update'}
                </button>
              {/if}
            </div>
          {/if}

          {#if updateProgress}
            <div class="update-progress" aria-live="polite">
              <div><span>{updateProgress.message}</span><strong>{Math.round(updateProgress.progress * 100)}%</strong></div>
              <div class="progress-track"><div style={`width:${Math.max(0, Math.min(100, updateProgress.progress * 100))}%`}></div></div>
              {#if updateProgress.totalBytes > 0}<small>{formatBytes(updateProgress.downloadedBytes)} / {formatBytes(updateProgress.totalBytes)}</small>{/if}
            </div>
          {/if}

          {#if updateError}<div class="notice error" role="alert">{updateError}</div>{/if}
          {#if updateMessage}<div class="notice success" role="status">{updateMessage}</div>{/if}
          {#if downloadedUpdate && !updateBusy}
            <button class="secondary compact" type="button" on:click={reopenDownloadedUpdate}>Open verified installer again</button>
          {/if}
          {#if platform === 'macos'}
            <p class="muted small">The updater verifies the downloaded DMG against the SHA-256 published by GitHub Releases, then opens the normal macOS installer. Because the current app is not Apple-notarized, macOS may still require Privacy & Security → Open Anyway after replacement.</p>
          {/if}
        </section>

        <section class="storage-card">
          <div class="storage-heading">
            <div>
              <span>Local storage used by Still2Solid</span>
              <strong>{summary?.nativeAvailable ? formatBytes(summary.totalRemovableBytes) : 'Desktop build required'}</strong>
            </div>
            <button class="secondary compact" type="button" disabled={busy} on:click={refresh}>Refresh</button>
          </div>

          {#if summary?.nativeAvailable}
            <div class="storage-grid">
              <div><span>Downloaded models</span><strong>{formatBytes(summary.modelsBytes)}</strong><small>{summary.installedModelDirectories} local model {summary.installedModelDirectories === 1 ? 'directory' : 'directories'}</small></div>
              <div><span>Temporary work</span><strong>{formatBytes(summary.temporaryBytes)}</strong><small>Abandoned jobs or interrupted installs</small></div>
              <div><span>Cache</span><strong>{formatBytes(summary.cacheBytes)}</strong><small>Reclaimable Still2Solid cache, including downloaded update installers</small></div>
              <div><span>Other app data</span><strong>{formatBytes(summary.otherAppDataBytes)}</strong><small>Local Still2Solid data outside model storage</small></div>
            </div>
          {:else}
            <p class="muted">Storage management is available in the installed Tauri desktop application, not the browser-only development preview.</p>
          {/if}
        </section>

        {#if error}<div class="notice error" role="alert">{error}</div>{/if}
        {#if message}<div class="notice success" role="status">{message}</div>{/if}
        {#if disabled}<div class="notice caution">Finish or cancel the active generation before changing local storage.</div>{/if}

        <section class="action-card">
          <div>
            <h3>Downloaded models</h3>
            <p>Remove TripoSR and Stable Fast 3D runtimes and weights without removing the Still2Solid application. Models can be installed again later.</p>
          </div>
          <button class="secondary" type="button" disabled={busy || disabled || !summary?.nativeAvailable || !summary?.modelsBytes} on:click={removeModels}>
            {busy ? 'Working…' : 'Remove downloaded models'}
          </button>
        </section>

        <section class="action-card">
          <div>
            <h3>Temporary files</h3>
            <p>Clear Still2Solid cache plus abandoned generation workspaces and interrupted model-install staging. Installed models and exported GLB, OBJ, STL or 3MF files are not touched.</p>
          </div>
          <button class="secondary" type="button" disabled={busy || disabled || !summary?.nativeAvailable || (!summary?.cacheBytes && !summary?.temporaryBytes)} on:click={clearCache}>
            Clear temporary files
          </button>
        </section>

        <section class="uninstall-card">
          <div>
            <span class="eyebrow">COMPLETE UNINSTALL</span>
            <h3>Prepare Still2Solid for uninstall</h3>
            <p>This removes downloaded models, Still2Solid app data, cache, abandoned temporary work and local Still2Solid preferences. It does not delete exported GLB, OBJ, STL or 3MF files that you saved elsewhere.</p>
            <p>The application itself stays in Applications because macOS does not provide apps with an uninstall callback when they are moved to Trash.</p>
          </div>

          {#if !confirmUninstall}
            <button class="danger" type="button" disabled={busy || disabled || !summary?.nativeAvailable} on:click={() => (confirmUninstall = true)}>
              Prepare for uninstall…
            </button>
          {:else}
            <div class="confirm-box">
              <strong>Remove Still2Solid's local data?</strong>
              <span>Downloaded models may use several gigabytes. This cleanup cannot be undone, although models can be downloaded again later.</span>
              <div class="button-row">
                <button class="secondary" type="button" disabled={busy} on:click={() => (confirmUninstall = false)}>Cancel</button>
                <button class="danger" type="button" disabled={busy || disabled} on:click={prepareForUninstall}>{busy ? 'Removing…' : 'Remove local data'}</button>
              </div>
            </div>
          {/if}

          {#if prepared && platform === 'macos'}
            <button class="primary" type="button" on:click={openApplications}>Open Applications folder</button>
          {/if}
        </section>
      </div>
    </section>
  </div>
{/if}

<style>
  .settings-layer { position: fixed; inset: 0; z-index: 55; display: grid; place-items: center; padding: 24px; }
  .backdrop { position: absolute; inset: 0; width: 100%; height: 100%; border: 0; background: rgba(4,6,10,.76); backdrop-filter: blur(8px); }
  .settings-panel { position: relative; width: min(780px,100%); max-height: calc(100vh - 48px); overflow: auto; border: 1px solid var(--border); border-radius: 22px; background: #13161c; box-shadow: 0 28px 90px rgba(0,0,0,.52); }
  header { position: sticky; top: 0; z-index: 2; display: flex; align-items: center; justify-content: space-between; gap: 20px; padding: 20px 22px; border-bottom: 1px solid var(--border); background: rgba(19,22,28,.96); backdrop-filter: blur(10px); }
  h2 { margin: 3px 0 0; font-size: 24px; }
  h3 { margin: 0 0 7px; font-size: 16px; }
  p { margin: 0; color: var(--muted); line-height: 1.55; font-size: 13px; }
  .eyebrow { color: var(--muted); font-size: 11px; letter-spacing: .12em; font-weight: 700; }
  .content { display: grid; gap: 12px; padding: 20px 22px 24px; }
  .storage-card, .action-card, .uninstall-card, .update-card { border: 1px solid var(--border); border-radius: 16px; background: var(--panel); padding: 17px; }
  .update-card { display: grid; gap: 13px; border-color: #3c4e7e; background: #171d2a; }
  .update-heading, .update-status { display: flex; align-items: center; justify-content: space-between; gap: 18px; }
  .update-heading > div { max-width: 540px; }
  .update-status { padding: 12px; border: 1px solid var(--border); border-radius: 12px; background: #101319; }
  .update-status.available { border-color: #496d5c; background: #16251d; }
  .update-status > div { display: grid; gap: 3px; }
  .update-status span, .update-status small, .update-progress span, .update-progress small { color: var(--muted); font-size: 12px; }
  .update-status strong { font-size: 18px; }
  .update-progress { display: grid; gap: 7px; }
  .update-progress > div:first-child { display: flex; justify-content: space-between; gap: 12px; }
  .progress-track { height: 7px; overflow: hidden; border-radius: 999px; background: #252a35; }
  .progress-track div { height: 100%; background: var(--accent); }
  .storage-heading, .action-card { display: flex; justify-content: space-between; align-items: center; gap: 18px; }
  .storage-heading > div { display: grid; gap: 5px; }
  .storage-heading span, .storage-grid span, .storage-grid small { color: var(--muted); font-size: 12px; }
  .storage-heading strong { font-size: 22px; }
  .storage-grid { display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: 10px; margin-top: 15px; }
  .storage-grid > div { display: grid; gap: 5px; padding: 12px; border: 1px solid var(--border); border-radius: 12px; background: #101319; }
  .storage-grid strong { font-size: 16px; }
  .action-card > div { max-width: 520px; }
  .uninstall-card { display: grid; gap: 14px; border-color: #5b3a3f; background: #1d1518; }
  .uninstall-card > div:first-child { display: grid; gap: 7px; }
  .confirm-box { display: grid; gap: 9px; padding: 13px; border: 1px solid #70464c; border-radius: 12px; background: #27191d; }
  .confirm-box span { color: #d7b8bc; font-size: 12px; line-height: 1.5; }
  .button-row { display: flex; justify-content: flex-end; gap: 8px; margin-top: 3px; }
  button { border-radius: 10px; padding: 9px 13px; font: inherit; font-size: 12px; font-weight: 700; cursor: pointer; }
  button:disabled { opacity: .45; cursor: not-allowed; }
  .secondary { border: 1px solid var(--border); background: #191d25; color: var(--text); }
  .primary { border: 1px solid #4d8dff; background: #2f6fe4; color: white; justify-self: start; }
  .danger { border: 1px solid #8b4d57; background: #3a2025; color: #ffd5d9; justify-self: start; }
  .compact, .close { padding: 7px 10px; }
  .notice { padding: 11px 13px; border-radius: 12px; font-size: 12px; line-height: 1.45; }
  .notice.error { border: 1px solid #68454a; color: #ffc2c7; background: #281a1d; }
  .notice.success { border: 1px solid #365f51; color: #bdebdc; background: #14231e; }
  .notice.caution { border: 1px solid #5b4b31; color: #e5c38a; background: #211d16; }
  .muted { margin-top: 12px; }
  .small { font-size: 11px; }
  @media (max-width: 680px) {
    .settings-layer { padding: 10px; }
    .settings-panel { max-height: calc(100vh - 20px); }
    .storage-grid { grid-template-columns: 1fr; }
    .storage-heading, .action-card, .update-heading, .update-status { align-items: stretch; flex-direction: column; }
    .action-card button, .update-heading button, .update-status button { align-self: start; }
  }
</style>