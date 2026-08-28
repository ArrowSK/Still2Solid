<script lang="ts">
  import { onMount } from 'svelte';
  import * as THREE from 'three';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
  import { GLTFExporter } from 'three/examples/jsm/exporters/GLTFExporter.js';
  import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';

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

  function loadProductionModel(url: string) {
    if (!scene || !url || loadedModelUrl === url) return;
    loadedModelUrl = url;
    viewerError = '';
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

  async function exportGlb() {
    if (modelUrl) {
      try {
        const response = await fetch(modelUrl);
        const blob = await response.blob();
        const url = URL.createObjectURL(blob);
        const anchor = document.createElement('a');
        anchor.href = url;
        anchor.download = exportFilename || 'still2solid.glb';
        anchor.click();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
      } catch (caught) {
        viewerError = caught instanceof Error ? caught.message : 'Could not export the generated GLB.';
      }
      return;
    }

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
  <div class="viewer-toolbar">
    <span>Drag to rotate · scroll to zoom</span>
    <button type="button" class="secondary" on:click={exportGlb}>{modelUrl ? 'Export GLB' : 'Export mock GLB'}</button>
  </div>
</div>

<style>
  .viewer-shell { border: 1px solid var(--border); border-radius: 18px; overflow: hidden; background: #15181f; }
  .viewer { width: 100%; min-height: 320px; }
  .viewer :global(canvas) { display: block; width: 100%; }
  .viewer-error { padding: 9px 14px; border-top: 1px solid #5c4141; color: #ffc0c0; background: #24191b; font-size: 12px; }
  .viewer-toolbar { display: flex; gap: 16px; align-items: center; justify-content: space-between; padding: 10px 14px; border-top: 1px solid var(--border); color: var(--muted); font-size: 13px; }
  @media (max-width: 620px) { .viewer-toolbar { align-items: stretch; flex-direction: column; } }
</style>
