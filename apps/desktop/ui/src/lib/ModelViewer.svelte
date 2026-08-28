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

  export let textureUrl = '';
  export let modelUrl = '';
  export let exportFilename = 'still2solid.glb';
  export let wireframe = false;
  export let showGrid = true;

  let host: HTMLDivElement;
  let renderer: THREE.WebGLRenderer;
  let scene: THREE.Scene;
  let camera: THREE.PerspectiveCamera;
  let controls: OrbitControls;
  let mockModel: THREE.Mesh | null = null;
  let productionModel: THREE.Object3D | null = null;
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

  function disposeMaterial(material: THREE.Material) {
    const candidate = material as THREE.MeshStandardMaterial;
    candidate.map?.dispose();
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

  function showMock() {
    loadedModelUrl = '';
    viewerError = '';
    inspection = null;
    inspectionUrl = '';
    exportOpen = false;
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
    setWireframe(productionModel ?? mockModel);
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
    <span>Drag to rotate · scroll to zoom</span>
    {#if modelUrl}
      <button type="button" class="secondary" on:click={() => (exportOpen = !exportOpen)}>{exportOpen ? 'Close export' : 'Export…'}</button>
    {:else}
      <button type="button" class="secondary" on:click={exportMockGlb}>Export mock GLB</button>
    {/if}
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
          <div><strong>STL</strong><span>Geometry only</span></div>
          <p>Binary STL for downstream mesh tools. STL stores no colour, texture or reliable unit metadata; print sizing and repair belong to M6.</p>
          <button type="button" class="secondary" disabled={!!exportBusy} on:click={() => runExport('stl')}>{exportBusy === 'stl' ? 'Converting…' : 'Export STL'}</button>
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
  .asset-facts { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 5px 12px; }
  .viewer-toolbar { display: flex; gap: 16px; align-items: center; justify-content: space-between; padding: 10px 14px; border-top: 1px solid var(--border); color: var(--muted); font-size: 13px; }
  .export-panel { padding: 18px; border-top: 1px solid var(--border); background: #11141a; }
  .export-intro { display: flex; align-items: flex-start; justify-content: space-between; gap: 24px; }
  .export-intro h3 { margin: 3px 0 0; color: var(--text); font-size: 18px; }
  .export-intro p { max-width: 470px; margin: 0; color: var(--muted); font-size: 12px; line-height: 1.5; }
  .export-eyebrow { color: var(--muted); font-size: 10px; font-weight: 700; letter-spacing: .1em; }
  .export-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin-top: 16px; }
  .export-grid article { display: flex; flex-direction: column; min-width: 0; padding: 13px; border: 1px solid var(--border); border-radius: 13px; background: #171a21; }
  .export-grid article > div { display: flex; align-items: baseline; justify-content: space-between; gap: 10px; }
  .export-grid article strong { font-size: 14px; }
  .export-grid article span { color: var(--muted); font-size: 10px; }
  .export-grid article p { flex: 1; margin: 9px 0 13px; color: var(--muted); font-size: 11px; line-height: 1.45; }
  .export-grid article button { width: 100%; padding: 9px 10px; }
  .inspection-grid { display: grid; grid-template-columns: repeat(5, 1fr); gap: 7px; margin-top: 12px; }
  .inspection-grid > div { display: grid; gap: 3px; padding: 8px 9px; border: 1px solid var(--border); border-radius: 9px; background: #15181e; }
  .inspection-grid span { color: var(--muted); font-size: 9px; text-transform: uppercase; letter-spacing: .07em; }
  .inspection-grid strong { font-size: 11px; }
  @media (max-width: 760px) {
    .export-grid { grid-template-columns: 1fr; }
    .inspection-grid { grid-template-columns: repeat(2, 1fr); }
    .export-intro { flex-direction: column; gap: 9px; }
  }
  @media (max-width: 620px) {
    .viewer-toolbar, .asset-strip { align-items: stretch; flex-direction: column; }
    .asset-facts { justify-content: flex-start; }
  }
</style>
