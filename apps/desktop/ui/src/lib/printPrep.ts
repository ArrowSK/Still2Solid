import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { strToU8, zipSync } from 'fflate';
import { safeAssetBaseName, validateGlbHeader } from './assetExport';

export type QuarterTurn = 0 | 90 | 180 | 270;

export interface PrintPrepOptions {
  targetMaxDimensionMm: number;
  rotateX: QuarterTurn;
  rotateY: QuarterTurn;
  rotateZ: QuarterTurn;
  flatBaseDepthMm: number;
  capSmallPlanarHoles: boolean;
}

export interface IndexedPrintMesh {
  vertices: Array<[number, number, number]>;
  triangles: Array<[number, number, number]>;
}

export interface PrintMeshAnalysis {
  vertices: number;
  triangles: number;
  degenerateTriangles: number;
  boundaryEdges: number;
  nonManifoldEdges: number;
  nonManifoldVertices: number;
  orientationConflicts: number;
  components: number;
  watertight: boolean;
  manifold: boolean;
  volumeMm3: number | null;
}

export interface PreparedPrintMesh {
  mesh: IndexedPrintMesh;
  boundsMm: { x: number; y: number; z: number };
  before: PrintMeshAnalysis;
  after: PrintMeshAnalysis;
  status: 'printable' | 'repair-incomplete';
  repairs: string[];
  warnings: string[];
  options: PrintPrepOptions;
}

interface EdgeOccurrence {
  face: number;
  from: number;
  to: number;
  direction: 1 | -1;
}

interface Topology {
  edges: Map<string, EdgeOccurrence[]>;
  faceAdjacency: Map<number, Set<number>>;
  components: number[][];
}

const DEG_EPSILON = 1e-12;

export function defaultPrintPrepOptions(): PrintPrepOptions {
  return {
    targetMaxDimensionMm: 100,
    rotateX: 0,
    rotateY: 0,
    rotateZ: 0,
    flatBaseDepthMm: 0,
    capSmallPlanarHoles: true,
  };
}

function ownedArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy.buffer;
}

function clampTargetSize(value: number): number {
  if (!Number.isFinite(value)) return 100;
  return Math.min(1000, Math.max(5, value));
}

function clampBaseDepth(value: number, target: number): number {
  if (!Number.isFinite(value) || value <= 0) return 0;
  return Math.min(value, Math.max(0, target * 0.1));
}

function normalizeQuarterTurn(value: number): QuarterTurn {
  const normalized = ((Math.round(value / 90) * 90) % 360 + 360) % 360;
  return (normalized === 0 || normalized === 90 || normalized === 180 || normalized === 270 ? normalized : 0) as QuarterTurn;
}

export function normalizePrintPrepOptions(options: PrintPrepOptions): PrintPrepOptions {
  const target = clampTargetSize(options.targetMaxDimensionMm);
  return {
    targetMaxDimensionMm: target,
    rotateX: normalizeQuarterTurn(options.rotateX),
    rotateY: normalizeQuarterTurn(options.rotateY),
    rotateZ: normalizeQuarterTurn(options.rotateZ),
    flatBaseDepthMm: clampBaseDepth(options.flatBaseDepthMm, target),
    capSmallPlanarHoles: !!options.capSmallPlanarHoles,
  };
}

function edgeKey(a: number, b: number): string {
  return a < b ? `${a}:${b}` : `${b}:${a}`;
}

function triangleAreaSquared(
  a: [number, number, number],
  b: [number, number, number],
  c: [number, number, number],
): number {
  const abx = b[0] - a[0];
  const aby = b[1] - a[1];
  const abz = b[2] - a[2];
  const acx = c[0] - a[0];
  const acy = c[1] - a[1];
  const acz = c[2] - a[2];
  const cx = aby * acz - abz * acy;
  const cy = abz * acx - abx * acz;
  const cz = abx * acy - aby * acx;
  return cx * cx + cy * cy + cz * cz;
}

function isDegenerate(mesh: IndexedPrintMesh, triangle: [number, number, number]): boolean {
  if (triangle[0] === triangle[1] || triangle[1] === triangle[2] || triangle[2] === triangle[0]) return true;
  return triangleAreaSquared(
    mesh.vertices[triangle[0]],
    mesh.vertices[triangle[1]],
    mesh.vertices[triangle[2]],
  ) <= DEG_EPSILON;
}

function buildTopology(mesh: IndexedPrintMesh): Topology {
  const edges = new Map<string, EdgeOccurrence[]>();
  const faceAdjacency = new Map<number, Set<number>>();

  mesh.triangles.forEach((triangle, face) => {
    faceAdjacency.set(face, new Set());
    const directed: Array<[number, number]> = [
      [triangle[0], triangle[1]],
      [triangle[1], triangle[2]],
      [triangle[2], triangle[0]],
    ];
    for (const [from, to] of directed) {
      const key = edgeKey(from, to);
      const occurrence: EdgeOccurrence = {
        face,
        from,
        to,
        direction: from < to ? 1 : -1,
      };
      const existing = edges.get(key);
      if (existing) existing.push(occurrence);
      else edges.set(key, [occurrence]);
    }
  });

  for (const occurrences of edges.values()) {
    for (let i = 0; i < occurrences.length; i += 1) {
      for (let j = i + 1; j < occurrences.length; j += 1) {
        faceAdjacency.get(occurrences[i].face)?.add(occurrences[j].face);
        faceAdjacency.get(occurrences[j].face)?.add(occurrences[i].face);
      }
    }
  }

  const components: number[][] = [];
  const visited = new Set<number>();
  for (let face = 0; face < mesh.triangles.length; face += 1) {
    if (visited.has(face)) continue;
    const stack = [face];
    const component: number[] = [];
    visited.add(face);
    while (stack.length) {
      const current = stack.pop()!;
      component.push(current);
      for (const neighbor of faceAdjacency.get(current) ?? []) {
        if (visited.has(neighbor)) continue;
        visited.add(neighbor);
        stack.push(neighbor);
      }
    }
    components.push(component);
  }

  return { edges, faceAdjacency, components };
}

function countNonManifoldVertices(mesh: IndexedPrintMesh, topology: Topology): number {
  const incidentFaces = new Map<number, number[]>();
  mesh.triangles.forEach((triangle, face) => {
    for (const vertex of triangle) {
      const list = incidentFaces.get(vertex);
      if (list) list.push(face);
      else incidentFaces.set(vertex, [face]);
    }
  });

  let nonManifold = 0;
  for (const [vertex, faces] of incidentFaces.entries()) {
    if (faces.length <= 1) continue;
    const faceSet = new Set(faces);
    const visited = new Set<number>();
    let fans = 0;
    for (const face of faces) {
      if (visited.has(face)) continue;
      fans += 1;
      const stack = [face];
      visited.add(face);
      while (stack.length) {
        const current = stack.pop()!;
        const tri = mesh.triangles[current];
        const currentEdges: Array<[number, number]> = [
          [tri[0], tri[1]],
          [tri[1], tri[2]],
          [tri[2], tri[0]],
        ];
        for (const [a, b] of currentEdges) {
          if (a !== vertex && b !== vertex) continue;
          for (const occurrence of topology.edges.get(edgeKey(a, b)) ?? []) {
            if (faceSet.has(occurrence.face) && !visited.has(occurrence.face)) {
              visited.add(occurrence.face);
              stack.push(occurrence.face);
            }
          }
        }
      }
    }
    if (fans > 1) nonManifold += 1;
  }
  return nonManifold;
}

function signedVolumeForFaces(mesh: IndexedPrintMesh, faces: number[]): number {
  let volume6 = 0;
  for (const face of faces) {
    const [ia, ib, ic] = mesh.triangles[face];
    const a = mesh.vertices[ia];
    const b = mesh.vertices[ib];
    const c = mesh.vertices[ic];
    volume6 += a[0] * (b[1] * c[2] - b[2] * c[1])
      - a[1] * (b[0] * c[2] - b[2] * c[0])
      + a[2] * (b[0] * c[1] - b[1] * c[0]);
  }
  return volume6 / 6;
}

export function analyzePrintMesh(mesh: IndexedPrintMesh): PrintMeshAnalysis {
  const topology = buildTopology(mesh);
  let degenerateTriangles = 0;
  for (const triangle of mesh.triangles) if (isDegenerate(mesh, triangle)) degenerateTriangles += 1;

  let boundaryEdges = 0;
  let nonManifoldEdges = 0;
  let orientationConflicts = 0;
  for (const occurrences of topology.edges.values()) {
    if (occurrences.length === 1) boundaryEdges += 1;
    else if (occurrences.length > 2) nonManifoldEdges += 1;
    if (occurrences.length === 2 && occurrences[0].direction === occurrences[1].direction) orientationConflicts += 1;
  }

  const nonManifoldVertices = countNonManifoldVertices(mesh, topology);
  const watertight = boundaryEdges === 0 && nonManifoldEdges === 0 && degenerateTriangles === 0;
  const manifold = nonManifoldEdges === 0 && nonManifoldVertices === 0;
  const volumeMm3 = watertight && manifold
    ? Math.abs(topology.components.reduce((sum, component) => sum + signedVolumeForFaces(mesh, component), 0))
    : null;

  return {
    vertices: mesh.vertices.length,
    triangles: mesh.triangles.length,
    degenerateTriangles,
    boundaryEdges,
    nonManifoldEdges,
    nonManifoldVertices,
    orientationConflicts,
    components: topology.components.length,
    watertight,
    manifold,
    volumeMm3,
  };
}

function cloneMesh(mesh: IndexedPrintMesh): IndexedPrintMesh {
  return {
    vertices: mesh.vertices.map((vertex) => [...vertex] as [number, number, number]),
    triangles: mesh.triangles.map((triangle) => [...triangle] as [number, number, number]),
  };
}

function removeDegenerateTriangles(mesh: IndexedPrintMesh): number {
  const before = mesh.triangles.length;
  mesh.triangles = mesh.triangles.filter((triangle) => !isDegenerate(mesh, triangle));
  return before - mesh.triangles.length;
}

function orientFacesConsistently(mesh: IndexedPrintMesh): { flippedFaces: number; inconsistent: boolean } {
  const topology = buildTopology(mesh);
  const desiredFlip = new Map<number, boolean>();
  let inconsistent = false;

  for (let seed = 0; seed < mesh.triangles.length; seed += 1) {
    if (desiredFlip.has(seed)) continue;
    desiredFlip.set(seed, false);
    const queue = [seed];
    while (queue.length) {
      const face = queue.shift()!;
      const currentFlip = desiredFlip.get(face)!;
      const triangle = mesh.triangles[face];
      const edges: Array<[number, number]> = [
        [triangle[0], triangle[1]],
        [triangle[1], triangle[2]],
        [triangle[2], triangle[0]],
      ];
      for (const [a, b] of edges) {
        const occurrences = topology.edges.get(edgeKey(a, b));
        if (!occurrences || occurrences.length !== 2) continue;
        const own = occurrences.find((item) => item.face === face)!;
        const other = occurrences[0].face === face ? occurrences[1] : occurrences[0];
        const mustDiffer = own.direction === other.direction;
        const neighborFlip = mustDiffer ? !currentFlip : currentFlip;
        const existing = desiredFlip.get(other.face);
        if (existing === undefined) {
          desiredFlip.set(other.face, neighborFlip);
          queue.push(other.face);
        } else if (existing !== neighborFlip) {
          inconsistent = true;
        }
      }
    }
  }

  let flippedFaces = 0;
  for (const [face, shouldFlip] of desiredFlip.entries()) {
    if (!shouldFlip) continue;
    const triangle = mesh.triangles[face];
    mesh.triangles[face] = [triangle[0], triangle[2], triangle[1]];
    flippedFaces += 1;
  }
  return { flippedFaces, inconsistent };
}

function orientClosedComponentsOutward(mesh: IndexedPrintMesh): number {
  const topology = buildTopology(mesh);
  let flippedComponents = 0;
  for (const component of topology.components) {
    const componentFaces = new Set(component);
    let closed = true;
    for (const occurrences of topology.edges.values()) {
      const touching = occurrences.filter((item) => componentFaces.has(item.face));
      if (touching.length && touching.length !== 2) {
        closed = false;
        break;
      }
    }
    if (!closed) continue;
    if (signedVolumeForFaces(mesh, component) >= 0) continue;
    for (const face of component) {
      const triangle = mesh.triangles[face];
      mesh.triangles[face] = [triangle[0], triangle[2], triangle[1]];
    }
    flippedComponents += 1;
  }
  return flippedComponents;
}

interface BoundaryLoop {
  vertices: number[];
  directedEdges: Array<[number, number]>;
}

function findBoundaryLoops(mesh: IndexedPrintMesh): BoundaryLoop[] {
  const topology = buildTopology(mesh);
  const boundary = Array.from(topology.edges.values())
    .filter((occurrences) => occurrences.length === 1)
    .map((occurrences) => occurrences[0]);
  const outgoing = new Map<number, EdgeOccurrence[]>();
  const incoming = new Map<number, EdgeOccurrence[]>();
  for (const edge of boundary) {
    const out = outgoing.get(edge.from);
    if (out) out.push(edge); else outgoing.set(edge.from, [edge]);
    const inc = incoming.get(edge.to);
    if (inc) inc.push(edge); else incoming.set(edge.to, [edge]);
  }

  const usable = boundary.filter((edge) => outgoing.get(edge.from)?.length === 1 && incoming.get(edge.from)?.length === 1);
  const usableKeys = new Set(usable.map((edge) => `${edge.from}>${edge.to}`));
  const used = new Set<string>();
  const loops: BoundaryLoop[] = [];

  for (const startEdge of usable) {
    const startKey = `${startEdge.from}>${startEdge.to}`;
    if (used.has(startKey)) continue;
    const vertices = [startEdge.from];
    const directedEdges: Array<[number, number]> = [];
    let edge: EdgeOccurrence | undefined = startEdge;
    let safety = 0;
    let closed = false;
    while (edge && safety < boundary.length + 2) {
      safety += 1;
      const key = `${edge.from}>${edge.to}`;
      if (!usableKeys.has(key) || used.has(key)) break;
      used.add(key);
      directedEdges.push([edge.from, edge.to]);
      vertices.push(edge.to);
      if (edge.to === startEdge.from) {
        closed = true;
        break;
      }
      edge = outgoing.get(edge.to)?.[0];
    }
    if (closed && vertices.length >= 4) {
      vertices.pop();
      loops.push({ vertices, directedEdges });
    }
  }
  return loops;
}

function loopMetrics(mesh: IndexedPrintMesh, loop: BoundaryLoop): { planarError: number; perimeter: number; baseLoop: boolean; centroid: [number, number, number] } {
  const points = loop.vertices.map((index) => mesh.vertices[index]);
  const centroid: [number, number, number] = [0, 0, 0];
  for (const point of points) {
    centroid[0] += point[0]; centroid[1] += point[1]; centroid[2] += point[2];
  }
  centroid[0] /= points.length; centroid[1] /= points.length; centroid[2] /= points.length;

  let nx = 0; let ny = 0; let nz = 0; let perimeter = 0;
  for (let index = 0; index < points.length; index += 1) {
    const current = points[index];
    const next = points[(index + 1) % points.length];
    nx += (current[1] - next[1]) * (current[2] + next[2]);
    ny += (current[2] - next[2]) * (current[0] + next[0]);
    nz += (current[0] - next[0]) * (current[1] + next[1]);
    perimeter += Math.hypot(next[0] - current[0], next[1] - current[1], next[2] - current[2]);
  }
  const length = Math.hypot(nx, ny, nz) || 1;
  nx /= length; ny /= length; nz /= length;
  let planarError = 0;
  for (const point of points) {
    planarError = Math.max(planarError, Math.abs(
      (point[0] - centroid[0]) * nx + (point[1] - centroid[1]) * ny + (point[2] - centroid[2]) * nz,
    ));
  }
  const baseLoop = points.every((point) => Math.abs(point[2]) <= 0.075);
  return { planarError, perimeter, baseLoop, centroid };
}

function capConservativeBoundaryLoops(mesh: IndexedPrintMesh, targetSizeMm: number): number {
  const loops = findBoundaryLoops(mesh);
  let capped = 0;
  const planarityTolerance = Math.max(0.05, targetSizeMm * 0.001);

  for (const loop of loops) {
    if (loop.vertices.length < 3 || loop.vertices.length > 128) continue;
    const metrics = loopMetrics(mesh, loop);
    const smallEnough = metrics.perimeter <= targetSizeMm * 0.35;
    if (metrics.planarError > planarityTolerance || (!smallEnough && !metrics.baseLoop)) continue;
    const centerIndex = mesh.vertices.length;
    mesh.vertices.push(metrics.centroid);
    for (const [from, to] of loop.directedEdges) {
      mesh.triangles.push([to, from, centerIndex]);
    }
    capped += 1;
  }
  return capped;
}

export function repairPrintMesh(
  input: IndexedPrintMesh,
  targetSizeMm: number,
  capSmallPlanarHoles: boolean,
): { mesh: IndexedPrintMesh; repairs: string[] } {
  const mesh = cloneMesh(input);
  const repairs: string[] = [];

  const removed = removeDegenerateTriangles(mesh);
  if (removed) repairs.push(`Removed ${removed} degenerate ${removed === 1 ? 'face' : 'faces'}.`);

  const orientation = orientFacesConsistently(mesh);
  if (orientation.flippedFaces) repairs.push(`Reoriented ${orientation.flippedFaces} ${orientation.flippedFaces === 1 ? 'face' : 'faces'} for consistent winding.`);
  if (orientation.inconsistent) repairs.push('Some winding constraints conflict; automatic orientation could not fully resolve the mesh.');

  const outward = orientClosedComponentsOutward(mesh);
  if (outward) repairs.push(`Flipped ${outward} closed ${outward === 1 ? 'shell' : 'shells'} to outward orientation.`);

  if (capSmallPlanarHoles) {
    const capped = capConservativeBoundaryLoops(mesh, targetSizeMm);
    if (capped) {
      repairs.push(`Capped ${capped} simple planar ${capped === 1 ? 'boundary loop' : 'boundary loops'}.`);
      const secondOrientation = orientFacesConsistently(mesh);
      if (secondOrientation.flippedFaces) repairs.push(`Reoriented ${secondOrientation.flippedFaces} additional faces after hole capping.`);
      orientClosedComponentsOutward(mesh);
    }
  }

  return { mesh, repairs };
}

function rotatePoint(point: [number, number, number], axis: 'x' | 'y' | 'z', degrees: QuarterTurn): [number, number, number] {
  let [x, y, z] = point;
  const turns = degrees / 90;
  for (let turn = 0; turn < turns; turn += 1) {
    if (axis === 'x') [y, z] = [-z, y];
    else if (axis === 'y') [x, z] = [z, -x];
    else [x, y] = [-y, x];
  }
  return [x, y, z];
}

export function transformTriangleSoupForPrint(
  triangleSoup: Array<[number, number, number]>,
  rawOptions: PrintPrepOptions,
): { points: Array<[number, number, number]>; options: PrintPrepOptions } {
  const options = normalizePrintPrepOptions(rawOptions);
  if (!triangleSoup.length) throw new Error('The generated model contains no triangle geometry.');

  // glTF/Three.js is Y-up. Print exports are explicitly Z-up and dimensioned in millimetres.
  let points = triangleSoup.map(([x, y, z]) => [x, -z, y] as [number, number, number]);
  points = points.map((point) => rotatePoint(point, 'x', options.rotateX));
  points = points.map((point) => rotatePoint(point, 'y', options.rotateY));
  points = points.map((point) => rotatePoint(point, 'z', options.rotateZ));

  let minX = Infinity; let minY = Infinity; let minZ = Infinity;
  let maxX = -Infinity; let maxY = -Infinity; let maxZ = -Infinity;
  for (const [x, y, z] of points) {
    minX = Math.min(minX, x); minY = Math.min(minY, y); minZ = Math.min(minZ, z);
    maxX = Math.max(maxX, x); maxY = Math.max(maxY, y); maxZ = Math.max(maxZ, z);
  }
  const sourceMax = Math.max(maxX - minX, maxY - minY, maxZ - minZ);
  if (!Number.isFinite(sourceMax) || sourceMax <= 1e-12) throw new Error('The generated model has zero-size geometry.');
  const scale = options.targetMaxDimensionMm / sourceMax;
  points = points.map(([x, y, z]) => [x * scale, y * scale, z * scale]);

  minZ *= scale;
  const basePlane = minZ + options.flatBaseDepthMm;
  if (options.flatBaseDepthMm > 0) {
    points = points.map(([x, y, z]) => [x, y, z < basePlane ? basePlane : z]);
    minZ = basePlane;
  }
  points = points.map(([x, y, z]) => [x, y, z - minZ]);
  return { points, options };
}

export function indexTriangleSoup(points: Array<[number, number, number]>, targetSizeMm: number): IndexedPrintMesh {
  const tolerance = Math.max(1e-5, targetSizeMm * 1e-6);
  const inverse = 1 / tolerance;
  const vertices: Array<[number, number, number]> = [];
  const triangles: Array<[number, number, number]> = [];
  const lookup = new Map<string, number>();

  const indexFor = (point: [number, number, number]): number => {
    const key = `${Math.round(point[0] * inverse)},${Math.round(point[1] * inverse)},${Math.round(point[2] * inverse)}`;
    const existing = lookup.get(key);
    if (existing !== undefined) return existing;
    const index = vertices.length;
    vertices.push(point);
    lookup.set(key, index);
    return index;
  };

  for (let index = 0; index + 2 < points.length; index += 3) {
    triangles.push([indexFor(points[index]), indexFor(points[index + 1]), indexFor(points[index + 2])]);
  }
  return { vertices, triangles };
}

function printBounds(mesh: IndexedPrintMesh): { x: number; y: number; z: number } {
  if (!mesh.vertices.length) return { x: 0, y: 0, z: 0 };
  let minX = Infinity; let minY = Infinity; let minZ = Infinity;
  let maxX = -Infinity; let maxY = -Infinity; let maxZ = -Infinity;
  for (const [x, y, z] of mesh.vertices) {
    minX = Math.min(minX, x); minY = Math.min(minY, y); minZ = Math.min(minZ, z);
    maxX = Math.max(maxX, x); maxY = Math.max(maxY, y); maxZ = Math.max(maxZ, z);
  }
  return { x: maxX - minX, y: maxY - minY, z: maxZ - minZ };
}

async function loadTriangleSoup(modelUrl: string): Promise<Array<[number, number, number]>> {
  const response = await fetch(modelUrl);
  if (!response.ok) throw new Error(`Could not read the generated GLB (${response.status}).`);
  const bytes = new Uint8Array(await response.arrayBuffer());
  validateGlbHeader(bytes);
  const gltf = await new GLTFLoader().parseAsync(ownedArrayBuffer(bytes), '');
  gltf.scene.updateMatrixWorld(true);
  const points: Array<[number, number, number]> = [];
  const vector = new THREE.Vector3();

  gltf.scene.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    const position = object.geometry.getAttribute('position');
    if (!position) return;
    const index = object.geometry.index;
    const triangleCount = index ? Math.floor(index.count / 3) : Math.floor(position.count / 3);
    for (let triangle = 0; triangle < triangleCount; triangle += 1) {
      for (let corner = 0; corner < 3; corner += 1) {
        const positionIndex = index ? index.getX(triangle * 3 + corner) : triangle * 3 + corner;
        vector.fromBufferAttribute(position, positionIndex).applyMatrix4(object.matrixWorld);
        points.push([vector.x, vector.y, vector.z]);
      }
    }
  });
  return points;
}

export async function preparePrintMesh(modelUrl: string, rawOptions: PrintPrepOptions): Promise<PreparedPrintMesh> {
  const soup = await loadTriangleSoup(modelUrl);
  const transformed = transformTriangleSoupForPrint(soup, rawOptions);
  const indexed = indexTriangleSoup(transformed.points, transformed.options.targetMaxDimensionMm);
  const before = analyzePrintMesh(indexed);
  const repaired = repairPrintMesh(indexed, transformed.options.targetMaxDimensionMm, transformed.options.capSmallPlanarHoles);
  const after = analyzePrintMesh(repaired.mesh);
  const printable = after.triangles > 0
    && after.degenerateTriangles === 0
    && after.boundaryEdges === 0
    && after.nonManifoldEdges === 0
    && after.nonManifoldVertices === 0
    && after.orientationConflicts === 0;

  const warnings: string[] = [];
  if (after.components > 1) warnings.push(`The prepared file contains ${after.components} disconnected shells. This can be intentional, but verify them in the slicer.`);
  if (!after.watertight) warnings.push('Open boundaries remain; the mesh is not watertight.');
  if (!after.manifold) warnings.push('Non-manifold topology remains and may require manual repair.');
  if (after.orientationConflicts) warnings.push(`${after.orientationConflicts} winding ${after.orientationConflicts === 1 ? 'conflict remains' : 'conflicts remain'}.`);
  if (transformed.options.flatBaseDepthMm > 0) warnings.push(`The lowest ${transformed.options.flatBaseDepthMm.toFixed(1)} mm band was flattened on the prepared copy only.`);
  if (!printable) warnings.push('Still2Solid will allow export for manual repair, but does not label this mesh print-ready.');

  return {
    mesh: repaired.mesh,
    boundsMm: printBounds(repaired.mesh),
    before,
    after,
    status: printable ? 'printable' : 'repair-incomplete',
    repairs: repaired.repairs,
    warnings,
    options: transformed.options,
  };
}

function xmlEscape(value: string): string {
  return value.replace(/[&<>"']/g, (character) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;',
  }[character] ?? character));
}

function numberString(value: number): string {
  const rounded = Math.abs(value) < 5e-7 ? 0 : value;
  return rounded.toFixed(6).replace(/\.?0+$/, '');
}

export function build3mfPackage(prepared: PreparedPrintMesh, title = 'Still2Solid model'): Uint8Array {
  const vertices = prepared.mesh.vertices
    .map(([x, y, z]) => `        <vertex x="${numberString(x)}" y="${numberString(y)}" z="${numberString(z)}"/>`)
    .join('\n');
  const triangles = prepared.mesh.triangles
    .map(([v1, v2, v3]) => `        <triangle v1="${v1}" v2="${v2}" v3="${v3}"/>`)
    .join('\n');
  const model = `<?xml version="1.0" encoding="UTF-8"?>\n<model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">\n  <metadata name="Title">${xmlEscape(title)}</metadata>\n  <metadata name="Application">Still2Solid 0.6.0</metadata>\n  <metadata name="Description">Prepared locally from the canonical GLB. Units are millimetres.</metadata>\n  <resources>\n    <object id="1" type="model">\n      <mesh>\n      <vertices>\n${vertices}\n      </vertices>\n      <triangles>\n${triangles}\n      </triangles>\n      </mesh>\n    </object>\n  </resources>\n  <build>\n    <item objectid="1"/>\n  </build>\n</model>\n`;
  const contentTypes = `<?xml version="1.0" encoding="UTF-8"?>\n<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">\n  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>\n  <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>\n</Types>\n`;
  const relationships = `<?xml version="1.0" encoding="UTF-8"?>\n<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">\n  <Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>\n</Relationships>\n`;
  return zipSync({
    '[Content_Types].xml': strToU8(contentTypes),
    '_rels/.rels': strToU8(relationships),
    '3D/3dmodel.model': strToU8(model),
  }, { level: 6 });
}

export function buildBinaryPreparedStl(prepared: PreparedPrintMesh): Uint8Array {
  const triangleCount = prepared.mesh.triangles.length;
  const buffer = new ArrayBuffer(84 + triangleCount * 50);
  const bytes = new Uint8Array(buffer);
  const view = new DataView(buffer);
  const header = new TextEncoder().encode('Still2Solid prepared STL; coordinates scaled in millimetres; STL has no unit metadata');
  bytes.set(header.slice(0, 80), 0);
  view.setUint32(80, triangleCount, true);
  let offset = 84;
  const writeVector = (x: number, y: number, z: number) => {
    view.setFloat32(offset, x, true); view.setFloat32(offset + 4, y, true); view.setFloat32(offset + 8, z, true); offset += 12;
  };
  for (const [ia, ib, ic] of prepared.mesh.triangles) {
    const a = new THREE.Vector3(...prepared.mesh.vertices[ia]);
    const b = new THREE.Vector3(...prepared.mesh.vertices[ib]);
    const c = new THREE.Vector3(...prepared.mesh.vertices[ic]);
    const normal = b.clone().sub(a).cross(c.clone().sub(a)).normalize();
    writeVector(normal.x, normal.y, normal.z);
    writeVector(a.x, a.y, a.z);
    writeVector(b.x, b.y, b.z);
    writeVector(c.x, c.y, c.z);
    view.setUint16(offset, 0, true); offset += 2;
  }
  return bytes;
}

function triggerDownload(bytes: Uint8Array, type: string, filename: string) {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  const blob = new Blob([copy.buffer], { type });
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

export function exportPrepared3mf(prepared: PreparedPrintMesh, filename: string): void {
  const base = safeAssetBaseName(filename);
  triggerDownload(build3mfPackage(prepared, base), 'model/3mf', `${base}-prepared.3mf`);
}

export function exportPreparedStl(prepared: PreparedPrintMesh, filename: string): void {
  const base = safeAssetBaseName(filename);
  triggerDownload(buildBinaryPreparedStl(prepared), 'model/stl', `${base}-prepared-mm.stl`);
}
