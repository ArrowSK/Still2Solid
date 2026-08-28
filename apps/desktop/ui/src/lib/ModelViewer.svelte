<script lang="ts">
  import { onMount } from 'svelte';
  import * as THREE from 'three';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
  import { GLTFExporter } from 'three/examples/jsm/exporters/GLTFExporter.js';
  import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
  import {
    exportBinaryStl,
    exportCanonicalGlb,
    exportObjPackage,
    formatAssetBytes,
    inspectCanonicalGlb,
    safeAssetBaseName,
    type AssetExportFormat,
    type CanonicalAssetInspection,
  } from './assetExport';
  import {
    defaultPrintPrepOptions,
    exportPrepared3mf,
    exportPreparedStl,
    preparePrintMesh,
    type PreparedPrintMesh,
    type PrintPrepOptions,
    type QuarterTurn,
  } from './printPrep';

  export let textureUrl = '';
  export let modelUrl = '';
  export let exportFilename = 'still2solid.glb';
  export let wireframe = false;
  export let showGrid = true;

  const quarterTurns: QuarterTurn[] = [0, 90, 180, 270];

  let host: HTMLDivElement;
  let renderer: THREE.WebGLRenderer;
  let scene: THREE.Scene;
  let camera: THREE.PerspectiveCamera;
  let controls: OrbitControls;
  let mockModel: THREE.Mesh | null = null;
  let productionModel: THREE.Object3D | null = null;
  let preparedPreview: THREE.Mesh | null = null;
  let frame = 0;
  let appliedTexture: THREE.Texture | null = null;
  let loadedModelUrl = '';
  let viewerError = '';

  let exportOpen = false;
  let exportBusy: AssetExportFormat | '' = '';
  let exportError = '';
  let inspection: CanonicalAssetInspection | null = null;
  let inspectionUrl = '';
  let inspecting = false;

  let printOpen = false;
  let printBusy = false;
  let printExportBusy: '3mf' | 'stl' | '' = '';
  let printError = '';
  let printOptions: PrintPrepOptions = defaultPrintPrepOptions();
  let preparedPrint: PreparedPrintMesh | null = null;
  let preparedOptionsKey = '';

  $: currentPrintOptionsKey = JSON.stringify(printOptions);
  $: printDirty = !!preparedPrint && currentPrintOptionsKey !== preparedOptionsKey;

  function disposeMaterial(material: THREE.Material) {
    const candidate = material as THREE.MeshStandardMaterial;
    candidate.map?.dispose();
    candidate.normalMap?.dispose();
    material.dispose();
  }

  function disposeObject(root: THREE.Object3D) {
    root.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return;
      object.geometry?.dispose();
      if (Array.isArray(object.material)) object.material.forEach(disposeMaterial);
      else if (object.material) disposeMaterial(object.material);
    });
  }

  function removePreparedPreview() {
    if (!preparedPreview || !scene) return;
    scene.remove(preparedPreview);
    disposeObject(preparedPreview);
    preparedPreview = null;
  }

  function setWireframe(root: THREE.Object3D | null) {
    root?.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return;
      const materials = Array.isArray(object.material) ? object.material : [object.material];
      materials.forEach((material) => {
        if ('wireframe' in material) {
          (material as THREE.MeshStandardMaterial).wireframe = wireframe;
          material.needsUpdate = true;
        }
      });
    });
  }

  function applyMockMaterial() {
    if (!mockModel || modelUrl) return;
    if (appliedTexture) {
      appliedTexture.dispose();
      appliedTexture = null;
    }
    const current = mockModel.material;
    if (Array.isArray(current)) current.forEach(disposeMaterial);
    else current?.dispose();

    const material = new THREE.MeshStandardMaterial({
      color: textureUrl ? 0xffffff : 0xb8bec9,
      roughness: 0.72,
      metalness: 0.05,
      wireframe,
    });
    if (textureUrl) {
      new THREE.TextureLoader().load(textureUrl, (texture) => {
        appliedTexture = texture;
        texture.colorSpace = THREE.SRGBColorSpace;
        material.map = texture;
        material.needsUpdate = true;
      });
    }
    mockModel.material = material;
  }

  function fitCamera(root: THREE.Object3D) {
    const box = new THREE.Box3().setFromObject(root);
    if (box.isEmpty()) return;
    const size = box.getSize(new THREE.Vector3());
    const center = box.getCenter(new THREE.Vector3());
    const maxSize = Math.max(size.x, size.y, size.z, 0.25);
    const distance = maxSize / (2 * Math.tan(THREE.MathUtils.degToRad(camera.fov * 0.5))) * 1.55;
    controls.target.copy(center);
    camera.near = Math.max(0.001, distance / 100);
    camera.far = Math.max(100, distance * 20);
    camera.position.copy(center).add(new THREE.Vector3(distance * 0.8, distance * 0.55, distance));
    camera.updateProjectionMatrix();
    controls.update();
    const grid = scene.getObjectByName('grid');
    if (grid) grid.position.y = box.min.y - maxSize * 0.015;
  }

  function showCanonicalPreview() {
    removePreparedPreview();
    if (productionModel) {
      productionModel.visible = true;
      setWireframe(productionModel);
      fitCamera(productionModel);
    } else if (mockModel) {
      mockModel.visible = true;
      fitCamera(mockModel);
    }
  }

  function showPreparedPrintPreview(prepared: PreparedPrintMesh) {
    if (!scene) return;
    removePreparedPreview();
    const geometry = new THREE.BufferGeometry();
    const positions = new Float32Array(prepared.mesh.vertices.length * 3);
    prepared.mesh.vertices.forEach(([x, y, z], index) => {
      // Print files are Z-up. Convert back to Three.js Y-up for the preview only.
      positions[index * 3] = x;
      positions[index * 3 + 1] = z;
      positions[index * 3 + 2] = -y;
    });
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    geometry.setIndex(prepared.mesh.triangles.flat());
    geometry.computeVertexNormals();
    const material = new THREE.MeshStandardMaterial({
      color: 0xc8cfdd,
      roughness: 0.72,
      metalness: 0.03,
      wireframe,
    });
    preparedPreview = new THREE.Mesh(geometry, material);
    preparedPreview.name = 'prepared-print-preview';
    productionModel && (productionModel.visible = false);
    mockModel && (mockModel.visible = false);
    scene.add(preparedPreview);
    fitCamera(preparedPreview);
  }

  function showMock() {
    loadedModelUrl = '';
    viewerError = '';
    inspection = null;
    inspectionUrl = '';
    exportOpen = false;
    printOpen = false;
    preparedPrint = null;
    preparedOptionsKey = '';
    removePreparedPreview();
    if (productionModel) {
      scene.remove(productionModel);
      disposeObject(productionModel);
      productionModel = null;
    }
    if (mockModel) {
      mockModel.visible = true;
      fitCamera(mockModel);
      applyMockMaterial();
    }
  }

  async function inspectProductionAsset(url: string) {
    if (!url || inspectionUrl === url || inspecting) return;
    inspectionUrl = url;
    inspecting = true;
    inspection = null;
    try {
      const next = await inspectCanonicalGlb(url);
      if (url === modelUrl) inspection = next;
    } catch (caught) {
      if (url === modelUrl) viewerError = caught instanceof Error ? caught.message : 'Could not validate the generated GLB.';
      inspectionUrl = '';
    } finally {
      inspecting = false;
    }
  }

  function loadProductionModel(url: string) {
    if (!scene || !url || loadedModelUrl === url) return;
    loadedModelUrl = url;
    viewerError = '';
    exportError = '';
    printError = '';
    preparedPrint = null;
    preparedOptionsKey = '';
    removePreparedPreview();
    void inspectProductionAsset(url);
    const loader = new GLTFLoader();
    loader.load(
      url,
      (gltf) => {
        if (url !== modelUrl) {
          disposeObject(gltf.scene);
          return;
        }
        if (productionModel) {
          scene.remove(productionModel);
          disposeObject(productionModel);
        }
        productionModel = gltf.scene;
        mockModel && (mockModel.visible = false);
        scene.add(productionModel);
        setWireframe(productionModel);
        fitCamera(productionModel);
      },
      undefined,
      (caught) => {
        loadedModelUrl = '';
        viewerError = caught instanceof Error ? caught.message : 'Could not load the generated GLB preview.';
      },
    );
  }

  $: if (scene) {
    modelUrl;
    if (modelUrl) loadProductionModel(modelUrl);
    else if (loadedModelUrl) showMock();
  }

  $: if (mockModel) {
    textureUrl;
    if (!modelUrl) applyMockMaterial();
  }

  $: if (scene) {
    wireframe;
    setWireframe(preparedPreview ?? productionModel ?? mockModel);
  }

  async function exportMockGlb() {
    if (!mockModel) return;
    const exporter = new GLTFExporter();
    exporter.parse(
      mockModel,
      (output) => {
        if (!(output instanceof ArrayBuffer)) return;
        const blob = new Blob([output], { type: 'model/gltf-binary' });
        const url = URL.createObjectURL(blob);
        const anchor = document.createElement('a');
        anchor.href = url;
        anchor.download = 'still2solid-mock.glb';
        anchor.click();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
      },
      (error) => (viewerError = `Mock GLB export failed: ${error}`),
      { binary: true },
    );
  }

  async function runExport(format: AssetExportFormat) {
    if (!modelUrl || exportBusy) return;
    exportBusy = format;
    exportError = '';
    try {
      if (format === 'glb') await exportCanonicalGlb(modelUrl, exportFilename);
      else if (format === 'obj') await exportObjPackage(modelUrl, exportFilename);
      else await exportBinaryStl(modelUrl, exportFilename);
    } catch (caught) {
      exportError = caught instanceof Error ? caught.message : 'Export failed.';
    } finally {
      exportBusy = '';
    }
  }

  function toggleExport() {
    exportOpen = !exportOpen;
    if (exportOpen) {
      printOpen = false;
      showCanonicalPreview();
    }
  }

  function togglePrint() {
    printOpen = !printOpen;
    if (printOpen) {
      exportOpen = false;
      if (preparedPrint && !printDirty) showPreparedPrintPreview(preparedPrint);
    } else {
      showCanonicalPreview();
    }
  }

  async function runPrintPrep() {
    if (!modelUrl || printBusy) return;
    printBusy = true;
    printError = '';
    try {
      preparedPrint = await preparePrintMesh(modelUrl, printOptions);
      printOptions = preparedPrint.options;
      preparedOptionsKey = JSON.stringify(preparedPrint.options);
      showPreparedPrintPreview(preparedPrint);
    } catch (caught) {
      printError = caught instanceof Error ? caught.message : 'Print preparation failed.';
      preparedPrint = null;
      preparedOptionsKey = '';
      showCanonicalPreview();
    } finally {
      printBusy = false;
    }
  }

  function runPrintExport(format: '3mf' | 'stl') {
    if (!preparedPrint || printDirty || printExportBusy) return;
    printExportBusy = format;
    printError = '';
    try {
      if (format === '3mf') exportPrepared3mf(preparedPrint, exportFilename);
      else exportPreparedStl(preparedPrint, exportFilename);
    } catch (caught) {
      printError = caught instanceof Error ? caught.message : 'Print export failed.';
    } finally {
      printExportBusy = '';
    }
  }

  function formatVolume(value: number | null): string {
    if (value === null) return '—';
    if (value >= 1000) return `${(value / 1000).toFixed(value >= 10000 ? 0 : 1)} cm³`;
    return `${value.toFixed(value >= 100 ? 0 : 1)} mm³`;
  }

  onMount(() => {
    scene = new THREE.Scene();
    scene.background = new THREE.Color(0x15181f);

    camera = new THREE.PerspectiveCamera(42, 1, 0.1, 100);
    camera.position.set(2.6, 2.1, 3.2);

    renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    host.appendChild(renderer.domElement);

    controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.target.set(0, 0.15, 0);

    const geometry = new THREE.BoxGeometry(1.45, 1.8, 1.05, 4, 4, 4);
    mockModel = new THREE.Mesh(geometry, new THREE.MeshStandardMaterial());
    mockModel.rotation.y = -0.35;
    scene.add(mockModel);
    applyMockMaterial();

    scene.add(new THREE.HemisphereLight(0xffffff, 0x343947, 2.5));
    const key = new THREE.DirectionalLight(0xffffff, 3.2);
    key.position.set(3, 4, 2);
    scene.add(key);

    const grid = new THREE.GridHelper(8, 16, 0x3e4451, 0x292e38);
    grid.name = 'grid';
    grid.position.y = -0.91;
    scene.add(grid);

    const resize = () => {
      const width = Math.max(1, host.clientWidth);
      const height = Math.max(320, Math.round(width * 0.64));
      renderer.setSize(width, height, false);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
    };

    const observer = new ResizeObserver(resize);
    observer.observe(host);
    resize();
    if (modelUrl) loadProductionModel(modelUrl);

    const animate = () => {
      controls.update();
      const gridObject = scene.getObjectByName('grid');
      if (gridObject) gridObject.visible = showGrid;
      renderer.render(scene, camera);
      frame = requestAnimationFrame(animate);
    };
    animate();

    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
      controls.dispose();
      renderer.dispose();
      appliedTexture?.dispose();
      if (preparedPreview) disposeObject(preparedPreview);
      if (productionModel) disposeObject(productionModel);
      if (mockModel) disposeObject(mockModel);
      renderer.domElement.remove();
    };
  });
</script>

<div class="viewer-shell">
  <div class="viewer" bind:this={host} aria-label={modelUrl ? 'Interactive generated 3D preview' : 'Interactive mock 3D preview'}></div>
  {#if viewerError}<div class="viewer-error" role="status">{viewerError}</div>{/if}

  {#if modelUrl}
    <div class="asset-strip">
      <div class="asset-identity">
        <span>Canonical master</span>
        <strong>GLB 2.0</strong>
      </div>
      {#if inspection}
        <div class="asset-facts">
          <span>{formatAssetBytes(inspection.bytes)}</span>
          <span>{inspection.meshes} {inspection.meshes === 1 ? 'mesh' : 'meshes'}</span>
          <span>{inspection.materials} {inspection.materials === 1 ? 'material' : 'materials'}</span>
          <span>{inspection.textures} {inspection.textures === 1 ? 'texture' : 'textures'}</span>
        </div>
      {:else if inspecting}
        <div class="asset-facts"><span>Validating local asset…</span></div>
      {/if}
    </div>
  {/if}

  <div class="viewer-toolbar">
    <span>{preparedPreview ? 'Prepared print preview · drag to rotate · scroll to zoom' : 'Drag to rotate · scroll to zoom'}</span>
    <div class="toolbar-actions">
      {#if modelUrl}
        <button type="button" class="secondary" class:active={printOpen} on:click={togglePrint}>{printOpen ? 'Close print prep' : 'Prepare for print…'}</button>
        <button type="button" class="secondary" class:active={exportOpen} on:click={toggleExport}>{exportOpen ? 'Close export' : 'Export…'}</button>
      {:else}
        <button type="button" class="secondary" on:click={exportMockGlb}>Export mock GLB</button>
      {/if}
    </div>
  </div>

  {#if modelUrl && exportOpen}
    <section class="export-panel" aria-label="Export generated model">
      <div class="export-intro">
        <div>
          <span class="export-eyebrow">M5 · NON-DESTRUCTIVE EXPORT</span>
          <h3>Export {safeAssetBaseName(exportFilename)}</h3>
        </div>
        <p>Still2Solid keeps the generated GLB as the canonical master. Other formats are derived in memory and never replace it.</p>
      </div>

      <div class="export-grid">
        <article>
          <div><strong>GLB</strong><span>Best fidelity</span></div>
          <p>Downloads the exact validated master produced by the local model, with embedded materials and textures unchanged.</p>
          <button type="button" class="primary" disabled={!!exportBusy} on:click={() => runExport('glb')}>{exportBusy === 'glb' ? 'Exporting…' : 'Export GLB'}</button>
        </article>

        <article>
          <div><strong>OBJ package</strong><span>Compatibility</span></div>
          <p>ZIP containing OBJ, MTL, PNG base-colour/normal textures when available, plus an asset manifest. Legacy MTL cannot reproduce full PBR materials.</p>
          <button type="button" class="secondary" disabled={!!exportBusy} on:click={() => runExport('obj')}>{exportBusy === 'obj' ? 'Converting…' : 'Export OBJ ZIP'}</button>
        </article>

        <article>
          <div><strong>STL</strong><span>Raw geometry</span></div>
          <p>Binary STL derived directly from the canonical GLB. It carries no colour, physical scale or reliable unit metadata. Use Print Prep when sizing matters.</p>
          <button type="button" class="secondary" disabled={!!exportBusy} on:click={() => runExport('stl')}>{exportBusy === 'stl' ? 'Converting…' : 'Export raw STL'}</button>
        </article>
      </div>

      {#if inspection}
        <div class="inspection-grid">
          <div><span>Vertices</span><strong>{inspection.vertices.toLocaleString()}</strong></div>
          <div><span>Triangles</span><strong>{inspection.triangles.toLocaleString()}</strong></div>
          <div><span>Bounds X</span><strong>{inspection.size.x.toFixed(3)}</strong></div>
          <div><span>Bounds Y</span><strong>{inspection.size.y.toFixed(3)}</strong></div>
          <div><span>Bounds Z</span><strong>{inspection.size.z.toFixed(3)}</strong></div>
        </div>
      {/if}

      {#if exportError}<div class="export-error" role="status">{exportError}</div>{/if}
    </section>
  {/if}

  {#if modelUrl && printOpen}
    <section class="print-panel" aria-label="Prepare generated model for 3D printing">
      <div class="print-intro">
        <div>
          <span class="export-eyebrow">M6 · PRINT PREPARATION</span>
          <h3>Prepare a printable copy</h3>
        </div>
        <p>The source photo provides no physical scale. Choose the final longest dimension explicitly; the prepared copy is Z-up and placed on the build plate at Z=0. The canonical GLB is never modified.</p>
      </div>

      <div class="print-layout">
        <div class="print-controls">
          <label>
            <span>Longest dimension</span>
            <div class="input-with-unit"><input type="number" min="5" max="1000" step="1" bind:value={printOptions.targetMaxDimensionMm} disabled={printBusy} /><strong>mm</strong></div>
            <small>5–1000 mm · isotropic scaling</small>
          </label>

          <div class="rotation-grid">
            <label><span>Rotate X</span><select bind:value={printOptions.rotateX} disabled={printBusy}>{#each quarterTurns as turn}<option value={turn}>{turn}°</option>{/each}</select></label>
            <label><span>Rotate Y</span><select bind:value={printOptions.rotateY} disabled={printBusy}>{#each quarterTurns as turn}<option value={turn}>{turn}°</option>{/each}</select></label>
            <label><span>Rotate Z</span><select bind:value={printOptions.rotateZ} disabled={printBusy}>{#each quarterTurns as turn}<option value={turn}>{turn}°</option>{/each}</select></label>
          </div>
          <small class="control-note">Rotation is applied after converting the generated Y-up model to printer-friendly Z-up coordinates.</small>

          <label>
            <span>Flat base band</span>
            <div class="input-with-unit"><input type="number" min="0" max="100" step="0.5" bind:value={printOptions.flatBaseDepthMm} disabled={printBusy} /><strong>mm</strong></div>
            <small>0 = off. Flattens only the lowest band of the prepared copy; capped conservatively when possible.</small>
          </label>

          <label class="check-row"><input type="checkbox" bind:checked={printOptions.capSmallPlanarHoles} disabled={printBusy} /><span>Cap simple planar holes when safe</span></label>

          <button type="button" class="primary prepare-button" disabled={printBusy} on:click={runPrintPrep}>{printBusy ? 'Analyzing and repairing…' : preparedPrint ? 'Prepare again' : 'Analyze & prepare'}</button>
          {#if printDirty}<p class="dirty-note">Settings changed. Prepare again before exporting; the preview still shows the previous prepared copy.</p>{/if}
        </div>

        <div class="print-result">
          {#if preparedPrint}
            <div class="print-status" class:printable={preparedPrint.status === 'printable'} class:incomplete={preparedPrint.status !== 'printable'}>
              <div><span>Status</span><strong>{preparedPrint.status === 'printable' ? 'Printable' : 'Automatic repair incomplete'}</strong></div>
              <p>{preparedPrint.status === 'printable' ? 'Topology checks passed after conservative local repair.' : 'One or more topology checks still need attention in a dedicated mesh editor or slicer.'}</p>
            </div>

            <div class="dimension-strip">
              <div><span>X</span><strong>{preparedPrint.boundsMm.x.toFixed(1)} mm</strong></div>
              <div><span>Y</span><strong>{preparedPrint.boundsMm.y.toFixed(1)} mm</strong></div>
              <div><span>Z</span><strong>{preparedPrint.boundsMm.z.toFixed(1)} mm</strong></div>
              <div><span>Volume</span><strong>{formatVolume(preparedPrint.after.volumeMm3)}</strong></div>
            </div>

            <div class="topology-grid">
              <div><span>Watertight</span><strong class:good={preparedPrint.after.watertight}>{preparedPrint.after.watertight ? 'Yes' : 'No'}</strong></div>
              <div><span>Manifold</span><strong class:good={preparedPrint.after.manifold}>{preparedPrint.after.manifold ? 'Yes' : 'No'}</strong></div>
              <div><span>Boundary edges</span><strong>{preparedPrint.after.boundaryEdges}</strong></div>
              <div><span>Non-manifold edges</span><strong>{preparedPrint.after.nonManifoldEdges}</strong></div>
              <div><span>Non-manifold vertices</span><strong>{preparedPrint.after.nonManifoldVertices}</strong></div>
              <div><span>Winding conflicts</span><strong>{preparedPrint.after.orientationConflicts}</strong></div>
              <div><span>Degenerate faces</span><strong>{preparedPrint.after.degenerateTriangles}</strong></div>
              <div><span>Disconnected shells</span><strong>{preparedPrint.after.components}</strong></div>
            </div>

            <details class="repair-details">
              <summary>Repair details</summary>
              <div>
                <p><strong>Before:</strong> {preparedPrint.before.boundaryEdges} boundary edges · {preparedPrint.before.nonManifoldEdges} non-manifold edges · {preparedPrint.before.degenerateTriangles} degenerate faces · {preparedPrint.before.orientationConflicts} winding conflicts.</p>
                {#if preparedPrint.repairs.length}
                  <ul>{#each preparedPrint.repairs as repair}<li>{repair}</li>{/each}</ul>
                {:else}
                  <p>No topology changes were required.</p>
                {/if}
                {#if preparedPrint.warnings.length}
                  <ul class="warnings">{#each preparedPrint.warnings as warning}<li>{warning}</li>{/each}</ul>
                {/if}
              </div>
            </details>

            <div class="print-export-grid">
              <article>
                <div><strong>3MF</strong><span>Recommended for printing</span></div>
                <p>Stores explicit millimetre units and the prepared geometry. This is the preferred hand-off to a modern slicer.</p>
                <button type="button" class="primary" disabled={printDirty || !!printExportBusy} on:click={() => runPrintExport('3mf')}>{printExportBusy === '3mf' ? 'Exporting…' : 'Export 3MF'}</button>
              </article>
              <article>
                <div><strong>Prepared STL</strong><span>Legacy geometry</span></div>
                <p>Coordinates are scaled in millimetres, but STL itself has no unit metadata. Confirm “mm” when importing into a slicer.</p>
                <button type="button" class="secondary" disabled={printDirty || !!printExportBusy} on:click={() => runPrintExport('stl')}>{printExportBusy === 'stl' ? 'Exporting…' : 'Export prepared STL'}</button>
              </article>
            </div>
          {:else}
            <div class="print-placeholder">
              <strong>No prepared copy yet</strong>
              <p>Choose the intended size and orientation, then run Analyze & prepare. Still2Solid will remove degenerate faces, repair consistent winding, orient closed shells outward and optionally cap only simple planar holes.</p>
            </div>
          {/if}
        </div>
      </div>

      {#if printError}<div class="export-error" role="status">{printError}</div>{/if}
    </section>
  {/if}
</div>

<style>
  .viewer-shell { border: 1px solid var(--border); border-radius: 18px; overflow: hidden; background: #15181f; }
  .viewer { width: 100%; min-height: 320px; }
  .viewer :global(canvas) { display: block; width: 100%; }
  .viewer-error, .export-error { padding: 9px 14px; border-top: 1px solid #5c4141; color: #ffc0c0; background: #24191b; font-size: 12px; }
  .asset-strip { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 9px 14px; border-top: 1px solid var(--border); background: #12151b; }
  .asset-identity { display: flex; align-items: baseline; gap: 8px; }
  .asset-identity span, .asset-facts { color: var(--muted); font-size: 11px; }
  .asset-identity strong { color: var(--accent-strong); font-size: 12px; }
  .asset-facts { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 6px 12px; }
  .viewer-toolbar { display: flex; gap: 16px; align-items: center; justify-content: space-between; padding: 10px 14px; border-top: 1px solid var(--border); color: var(--muted); font-size: 13px; }
  .toolbar-actions { display: flex; gap: 8px; }
  .toolbar-actions button.active { border-color: #617bc6; color: var(--accent-strong); background: #222a3a; }

  .export-panel, .print-panel { border-top: 1px solid var(--border); background: #101319; }
  .export-panel { padding: 18px; }
  .export-intro, .print-intro { display: grid; grid-template-columns: minmax(220px, .7fr) 1.3fr; gap: 20px; align-items: end; }
  .export-intro h3, .print-intro h3 { margin: 4px 0 0; font-size: 18px; }
  .export-intro p, .print-intro p { margin: 0; color: var(--muted); font-size: 12px; line-height: 1.5; }
  .export-eyebrow { color: var(--muted); font-size: 10px; font-weight: 700; letter-spacing: .1em; }
  .export-grid, .print-export-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; margin-top: 16px; }
  .export-grid article, .print-export-grid article { display: flex; flex-direction: column; gap: 10px; min-width: 0; padding: 13px; border: 1px solid var(--border); border-radius: 13px; background: #171b22; }
  .export-grid article > div, .print-export-grid article > div { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; }
  .export-grid article span, .print-export-grid article span { color: var(--muted); font-size: 10px; }
  .export-grid article p, .print-export-grid article p { flex: 1; margin: 0; color: var(--muted); font-size: 11px; line-height: 1.45; }
  .export-grid button, .print-export-grid button { width: 100%; padding: 9px 11px; }
  .inspection-grid { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 8px; margin-top: 12px; }
  .inspection-grid div { display: grid; gap: 3px; padding: 8px 10px; border: 1px solid var(--border); border-radius: 9px; background: #13161c; }
  .inspection-grid span { color: var(--muted); font-size: 9px; text-transform: uppercase; letter-spacing: .06em; }
  .inspection-grid strong { font-size: 11px; }

  .print-panel { padding: 18px; }
  .print-layout { display: grid; grid-template-columns: minmax(260px, .7fr) minmax(0, 1.3fr); gap: 14px; margin-top: 16px; }
  .print-controls, .print-result { min-width: 0; border: 1px solid var(--border); border-radius: 14px; background: #151920; }
  .print-controls { display: grid; gap: 13px; padding: 14px; align-content: start; }
  .print-controls label:not(.check-row) { display: grid; gap: 6px; color: var(--muted); font-size: 11px; font-weight: 650; }
  .print-controls small, .control-note { color: #737d8e; font-size: 10px; line-height: 1.4; }
  .input-with-unit { display: grid; grid-template-columns: 1fr auto; overflow: hidden; border: 1px solid var(--border); border-radius: 9px; background: #0f1217; }
  .input-with-unit input { min-width: 0; border: 0; padding: 9px 10px; color: var(--text); background: transparent; outline: none; }
  .input-with-unit strong { display: grid; place-items: center; min-width: 42px; border-left: 1px solid var(--border); color: var(--muted); font-size: 10px; }
  .rotation-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 7px; }
  .rotation-grid select { width: 100%; border: 1px solid var(--border); border-radius: 8px; padding: 8px; color: var(--text); background: #0f1217; }
  .check-row { display: flex; align-items: center; gap: 8px; color: var(--text); font-size: 11px; }
  .prepare-button { width: 100%; margin-top: 2px; }
  .dirty-note { margin: 0; color: #dfc188; font-size: 10px; line-height: 1.4; }

  .print-result { padding: 14px; }
  .print-placeholder { display: grid; place-items: center; min-height: 260px; padding: 28px; text-align: center; }
  .print-placeholder strong { font-size: 15px; }
  .print-placeholder p { max-width: 480px; margin: 7px 0 0; color: var(--muted); font-size: 11px; line-height: 1.55; }
  .print-status { display: grid; grid-template-columns: auto 1fr; gap: 14px; align-items: center; padding: 12px; border: 1px solid var(--border); border-radius: 12px; }
  .print-status.printable { border-color: #3f6d58; background: #14231c; }
  .print-status.incomplete { border-color: #6d593b; background: #251f15; }
  .print-status > div { display: grid; gap: 3px; }
  .print-status span { color: var(--muted); font-size: 9px; text-transform: uppercase; letter-spacing: .08em; }
  .print-status strong { font-size: 13px; }
  .print-status.printable strong { color: #bde0cd; }
  .print-status.incomplete strong { color: #e5ca97; }
  .print-status p { margin: 0; color: var(--muted); font-size: 10px; line-height: 1.4; }
  .dimension-strip { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 7px; margin-top: 10px; }
  .dimension-strip div, .topology-grid div { display: grid; gap: 3px; padding: 8px 9px; border: 1px solid var(--border); border-radius: 9px; background: #11151a; }
  .dimension-strip span, .topology-grid span { color: var(--muted); font-size: 9px; }
  .dimension-strip strong, .topology-grid strong { font-size: 11px; }
  .topology-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 7px; margin-top: 7px; }
  .topology-grid strong.good { color: #bde0cd; }
  .repair-details { margin-top: 10px; border: 1px solid var(--border); border-radius: 10px; color: var(--accent-strong); font-size: 11px; }
  .repair-details summary { padding: 9px 10px; cursor: pointer; }
  .repair-details > div { padding: 0 10px 10px; color: var(--muted); line-height: 1.45; }
  .repair-details p { margin: 4px 0; }
  .repair-details ul { margin: 7px 0 0; padding-left: 18px; }
  .repair-details .warnings { color: #dfc188; }
  .print-export-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }

  @media (max-width: 800px) {
    .export-grid { grid-template-columns: 1fr; }
    .inspection-grid { grid-template-columns: repeat(2, 1fr); }
    .export-intro, .print-intro, .print-layout { grid-template-columns: 1fr; }
  }
  @media (max-width: 620px) {
    .viewer-toolbar { align-items: stretch; flex-direction: column; }
    .toolbar-actions { display: grid; grid-template-columns: 1fr 1fr; }
    .asset-strip { align-items: flex-start; flex-direction: column; }
    .asset-facts { justify-content: flex-start; }
    .rotation-grid, .topology-grid, .dimension-strip, .print-export-grid { grid-template-columns: 1fr 1fr; }
  }
</style>
