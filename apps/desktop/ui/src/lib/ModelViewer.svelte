<script lang="ts">
  import { onMount } from 'svelte';
  import * as THREE from 'three';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
  import { GLTFExporter } from 'three/examples/jsm/exporters/GLTFExporter.js';

  export let textureUrl = '';
  export let wireframe = false;
  export let showGrid = true;

  let host: HTMLDivElement;
  let renderer: THREE.WebGLRenderer;
  let scene: THREE.Scene;
  let camera: THREE.PerspectiveCamera;
  let controls: OrbitControls;
  let model: THREE.Mesh;
  let frame = 0;
  let appliedTexture: THREE.Texture | null = null;

  function applyMaterial() {
    if (!model) return;

    if (appliedTexture) {
      appliedTexture.dispose();
      appliedTexture = null;
    }

    const current = model.material;
    if (Array.isArray(current)) current.forEach((material) => material.dispose());
    else current?.dispose();

    const material = new THREE.MeshStandardMaterial({
      color: textureUrl ? 0xffffff : 0xb8bec9,
      roughness: 0.72,
      metalness: 0.05,
      wireframe,
    });

    if (textureUrl) {
      const loader = new THREE.TextureLoader();
      loader.load(textureUrl, (texture) => {
        appliedTexture = texture;
        texture.colorSpace = THREE.SRGBColorSpace;
        material.map = texture;
        material.needsUpdate = true;
      });
    }

    model.material = material;
  }

  $: if (model) {
    textureUrl;
    wireframe;
    applyMaterial();
  }

  function exportGlb() {
    if (!model) return;
    const exporter = new GLTFExporter();
    exporter.parse(
      model,
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
      (error) => console.error('Mock GLB export failed', error),
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
    model = new THREE.Mesh(geometry, new THREE.MeshStandardMaterial());
    model.rotation.y = -0.35;
    scene.add(model);
    applyMaterial();

    const hemi = new THREE.HemisphereLight(0xffffff, 0x343947, 2.5);
    scene.add(hemi);
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
      geometry.dispose();
      appliedTexture?.dispose();
      if (model.material instanceof THREE.Material) model.material.dispose();
      renderer.domElement.remove();
    };
  });
</script>

<div class="viewer-shell">
  <div class="viewer" bind:this={host} aria-label="Interactive mock 3D preview"></div>
  <div class="viewer-toolbar">
    <span>Drag to rotate · scroll to zoom</span>
    <button type="button" class="secondary" on:click={exportGlb}>Export mock GLB</button>
  </div>
</div>

<style>
  .viewer-shell { border: 1px solid var(--border); border-radius: 18px; overflow: hidden; background: #15181f; }
  .viewer { width: 100%; min-height: 320px; }
  .viewer :global(canvas) { display: block; width: 100%; }
  .viewer-toolbar { display: flex; gap: 16px; align-items: center; justify-content: space-between; padding: 10px 14px; border-top: 1px solid var(--border); color: var(--muted); font-size: 13px; }
  @media (max-width: 620px) { .viewer-toolbar { align-items: stretch; flex-direction: column; } }
</style>
