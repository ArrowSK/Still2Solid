import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { OBJExporter } from 'three/examples/jsm/exporters/OBJExporter.js';
import { STLExporter } from 'three/examples/jsm/exporters/STLExporter.js';
import { strToU8, zipSync } from 'fflate';

export type AssetExportFormat = 'glb' | 'obj' | 'stl';

export interface CanonicalAssetInspection {
  format: 'GLB';
  version: number;
  bytes: number;
  meshes: number;
  vertices: number;
  triangles: number;
  materials: number;
  textures: number;
  size: { x: number; y: number; z: number };
}

interface MaterialExportRecord {
  name: string;
  material: THREE.Material;
}

const GLB_MAGIC = 0x46546c67;

export function safeAssetBaseName(filename: string): string {
  const withoutExtension = filename.replace(/\.(glb|gltf|obj|stl|zip)$/i, '');
  const normalized = withoutExtension
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-zA-Z0-9._-]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 80);
  return normalized || 'still2solid-model';
}

export function validateGlbHeader(bytes: Uint8Array): { version: number; length: number } {
  if (bytes.byteLength < 12) throw new Error('Generated asset is too small to be a valid GLB.');
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (view.getUint32(0, true) !== GLB_MAGIC) throw new Error('Generated asset is not a GLB file.');
  const version = view.getUint32(4, true);
  const length = view.getUint32(8, true);
  if (version !== 2) throw new Error(`Unsupported GLB version ${version}; Still2Solid expects GLB 2.0.`);
  if (length !== bytes.byteLength) {
    throw new Error(`GLB length mismatch: header declares ${length} bytes, received ${bytes.byteLength}.`);
  }
  return { version, length };
}

export function formatAssetBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(bytes < 10 * 1024 * 1024 ? 1 : 0)} MB`;
}

function ownedArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy.buffer;
}

async function fetchCanonicalBytes(modelUrl: string): Promise<Uint8Array> {
  const response = await fetch(modelUrl);
  if (!response.ok) throw new Error(`Could not read the generated GLB (${response.status}).`);
  const bytes = new Uint8Array(await response.arrayBuffer());
  validateGlbHeader(bytes);
  return bytes;
}

async function loadCanonicalScene(modelUrl: string): Promise<{ scene: THREE.Group; bytes: Uint8Array }> {
  const bytes = await fetchCanonicalBytes(modelUrl);
  const loader = new GLTFLoader();
  const gltf = await loader.parseAsync(ownedArrayBuffer(bytes), '');
  gltf.scene.updateMatrixWorld(true);
  return { scene: gltf.scene, bytes };
}

function geometryTriangleCount(geometry: THREE.BufferGeometry): number {
  if (geometry.index) return Math.floor(geometry.index.count / 3);
  const position = geometry.getAttribute('position');
  return position ? Math.floor(position.count / 3) : 0;
}

function geometryVertexCount(geometry: THREE.BufferGeometry): number {
  return geometry.getAttribute('position')?.count ?? 0;
}

function materialTextures(material: THREE.Material): THREE.Texture[] {
  const candidate = material as THREE.MeshStandardMaterial & {
    normalMap?: THREE.Texture | null;
    alphaMap?: THREE.Texture | null;
    emissiveMap?: THREE.Texture | null;
  };
  return [candidate.map, candidate.normalMap, candidate.alphaMap, candidate.emissiveMap]
    .filter((texture): texture is THREE.Texture => !!texture);
}

export async function inspectCanonicalGlb(modelUrl: string): Promise<CanonicalAssetInspection> {
  const { scene, bytes } = await loadCanonicalScene(modelUrl);
  const header = validateGlbHeader(bytes);
  const materials = new Set<string>();
  const textures = new Set<string>();
  let meshes = 0;
  let vertices = 0;
  let triangles = 0;

  scene.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    meshes += 1;
    vertices += geometryVertexCount(object.geometry);
    triangles += geometryTriangleCount(object.geometry);
    const meshMaterials = Array.isArray(object.material) ? object.material : [object.material];
    for (const material of meshMaterials) {
      if (!material) continue;
      materials.add(material.uuid);
      for (const texture of materialTextures(material)) textures.add(texture.uuid);
    }
  });

  const box = new THREE.Box3().setFromObject(scene);
  const size = box.isEmpty() ? new THREE.Vector3() : box.getSize(new THREE.Vector3());

  return {
    format: 'GLB',
    version: header.version,
    bytes: bytes.byteLength,
    meshes,
    vertices,
    triangles,
    materials: materials.size,
    textures: textures.size,
    size: { x: size.x, y: size.y, z: size.z },
  };
}

function triggerDownload(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  anchor.style.display = 'none';
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1500);
}

export async function exportCanonicalGlb(modelUrl: string, filename: string): Promise<void> {
  const bytes = await fetchCanonicalBytes(modelUrl);
  triggerDownload(new Blob([ownedArrayBuffer(bytes)], { type: 'model/gltf-binary' }), `${safeAssetBaseName(filename)}.glb`);
}

function cloneSceneForObj(scene: THREE.Object3D): { root: THREE.Object3D; materials: MaterialExportRecord[] } {
  const root = scene.clone(true);
  const byOriginalUuid = new Map<string, THREE.Material>();
  const records: MaterialExportRecord[] = [];

  const materialClone = (material: THREE.Material): THREE.Material => {
    const existing = byOriginalUuid.get(material.uuid);
    if (existing) return existing;
    const clone = material.clone();
    clone.name = `material_${String(records.length + 1).padStart(2, '0')}`;
    byOriginalUuid.set(material.uuid, clone);
    records.push({ name: clone.name, material: clone });
    return clone;
  };

  root.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    if (Array.isArray(object.material)) object.material = object.material.map(materialClone);
    else if (object.material) object.material = materialClone(object.material);
  });
  root.updateMatrixWorld(true);
  return { root, materials: records };
}

function colorLine(color: THREE.Color | undefined): string {
  const value = color ?? new THREE.Color(0.8, 0.8, 0.8);
  return `${value.r.toFixed(6)} ${value.g.toFixed(6)} ${value.b.toFixed(6)}`;
}

async function textureToPng(texture: THREE.Texture): Promise<Uint8Array | null> {
  const image = texture.image as (CanvasImageSource & { width?: number; height?: number; data?: ArrayLike<number> }) | undefined;
  if (!image) return null;
  const width = Number(image.width ?? 0);
  const height = Number(image.height ?? 0);
  if (!width || !height) return null;

  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d');
  if (!context) return null;

  try {
    if ('data' in image && image.data && !(image instanceof HTMLImageElement)) {
      const source = image.data;
      const rgba = new Uint8ClampedArray(width * height * 4);
      const sourceLength = source.length;
      if (sourceLength === width * height * 4) {
        for (let index = 0; index < rgba.length; index += 1) rgba[index] = Number(source[index]);
      } else if (sourceLength === width * height * 3) {
        for (let pixel = 0; pixel < width * height; pixel += 1) {
          rgba[pixel * 4] = Number(source[pixel * 3]);
          rgba[pixel * 4 + 1] = Number(source[pixel * 3 + 1]);
          rgba[pixel * 4 + 2] = Number(source[pixel * 3 + 2]);
          rgba[pixel * 4 + 3] = 255;
        }
      } else {
        return null;
      }
      context.putImageData(new ImageData(rgba, width, height), 0, 0);
    } else {
      context.drawImage(image, 0, 0, width, height);
    }
  } catch {
    return null;
  }

  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
  return blob ? new Uint8Array(await blob.arrayBuffer()) : null;
}

async function buildMtlAndTextures(records: MaterialExportRecord[]): Promise<{ mtl: string; files: Record<string, Uint8Array> }> {
  const lines: string[] = ['# Still2Solid M5 OBJ material library', '# GLB PBR materials are approximated for legacy MTL.'];
  const files: Record<string, Uint8Array> = {};

  for (const record of records) {
    const material = record.material as THREE.MeshStandardMaterial;
    lines.push('', `newmtl ${record.name}`);
    lines.push(`Kd ${colorLine(material.color)}`);
    lines.push(`d ${(material.opacity ?? 1).toFixed(6)}`);
    lines.push('illum 2');

    if (material.map) {
      const texturePath = `textures/${record.name}-basecolor.png`;
      const bytes = await textureToPng(material.map);
      if (bytes) {
        files[texturePath] = bytes;
        lines.push(`map_Kd ${texturePath}`);
      }
    }
    if (material.normalMap) {
      const texturePath = `textures/${record.name}-normal.png`;
      const bytes = await textureToPng(material.normalMap);
      if (bytes) {
        files[texturePath] = bytes;
        lines.push(`map_Bump ${texturePath}`);
      }
    }
  }

  return { mtl: `${lines.join('\n')}\n`, files };
}

export async function exportObjPackage(modelUrl: string, filename: string): Promise<void> {
  const base = safeAssetBaseName(filename);
  const { scene, bytes } = await loadCanonicalScene(modelUrl);
  const { root, materials } = cloneSceneForObj(scene);
  const obj = new OBJExporter().parse(root);
  const materialBundle = await buildMtlAndTextures(materials);
  const mtlName = `${base}.mtl`;
  const objWithLibrary = `mtllib ${mtlName}\n${obj}`;
  const inspection = await inspectCanonicalGlb(modelUrl);

  const manifest = {
    generator: 'Still2Solid 0.5.0',
    canonicalSource: 'GLB 2.0',
    export: 'OBJ + MTL + PNG textures',
    originalGlbBytes: bytes.byteLength,
    meshes: inspection.meshes,
    vertices: inspection.vertices,
    triangles: inspection.triangles,
    materials: inspection.materials,
    textures: inspection.textures,
    note: 'OBJ/MTL cannot represent the full GLB/PBR material model. Base-colour and normal textures are exported when browser-readable; the canonical GLB remains the fidelity-preserving master.',
  };

  const zip = zipSync({
    [`${base}.obj`]: strToU8(objWithLibrary),
    [mtlName]: strToU8(materialBundle.mtl),
    'asset.json': strToU8(JSON.stringify(manifest, null, 2)),
    ...materialBundle.files,
  }, { level: 6 });

  triggerDownload(new Blob([ownedArrayBuffer(zip)], { type: 'application/zip' }), `${base}-obj.zip`);
}

export async function exportBinaryStl(modelUrl: string, filename: string): Promise<void> {
  const base = safeAssetBaseName(filename);
  const { scene } = await loadCanonicalScene(modelUrl);
  const output = new STLExporter().parse(scene, { binary: true }) as DataView;
  const source = new Uint8Array(output.buffer, output.byteOffset, output.byteLength);
  triggerDownload(new Blob([ownedArrayBuffer(source)], { type: 'model/stl' }), `${base}.stl`);
}
