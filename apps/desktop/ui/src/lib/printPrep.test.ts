import { describe, expect, it } from 'vitest';
import { strFromU8, unzipSync } from 'fflate';
import {
  analyzePrintMesh,
  build3mfPackage,
  buildBinaryPreparedStl,
  defaultPrintPrepOptions,
  indexTriangleSoup,
  repairPrintMesh,
  transformTriangleSoupForPrint,
  type IndexedPrintMesh,
  type PreparedPrintMesh,
} from './printPrep';

function tetrahedron(): IndexedPrintMesh {
  return {
    vertices: [
      [0, 0, 0],
      [10, 0, 0],
      [0, 10, 0],
      [0, 0, 10],
    ],
    triangles: [
      [0, 2, 1],
      [0, 1, 3],
      [0, 3, 2],
      [1, 2, 3],
    ],
  };
}

function prepared(mesh = tetrahedron()): PreparedPrintMesh {
  const analysis = analyzePrintMesh(mesh);
  return {
    mesh,
    boundsMm: { x: 10, y: 10, z: 10 },
    before: analysis,
    after: analysis,
    status: 'printable',
    repairs: [],
    warnings: [],
    options: defaultPrintPrepOptions(),
  };
}

describe('M6 print topology analysis', () => {
  it('recognises a closed manifold tetrahedron as printable topology', () => {
    const analysis = analyzePrintMesh(tetrahedron());
    expect(analysis.watertight).toBe(true);
    expect(analysis.manifold).toBe(true);
    expect(analysis.boundaryEdges).toBe(0);
    expect(analysis.nonManifoldEdges).toBe(0);
    expect(analysis.nonManifoldVertices).toBe(0);
    expect(analysis.orientationConflicts).toBe(0);
    expect(analysis.components).toBe(1);
    expect(analysis.volumeMm3).toBeGreaterThan(0);
  });

  it('reports open boundaries when one face is missing', () => {
    const mesh = tetrahedron();
    mesh.triangles.pop();
    const analysis = analyzePrintMesh(mesh);
    expect(analysis.watertight).toBe(false);
    expect(analysis.boundaryEdges).toBe(3);
  });

  it('repairs inconsistent winding without changing the canonical input', () => {
    const mesh = tetrahedron();
    mesh.triangles[0] = [0, 1, 2];
    expect(analyzePrintMesh(mesh).orientationConflicts).toBeGreaterThan(0);
    const repaired = repairPrintMesh(mesh, 100, false);
    expect(analyzePrintMesh(repaired.mesh).orientationConflicts).toBe(0);
    expect(analyzePrintMesh(repaired.mesh).watertight).toBe(true);
    expect(mesh.triangles[0]).toEqual([0, 1, 2]);
  });

  it('removes degenerate faces conservatively', () => {
    const mesh = tetrahedron();
    mesh.triangles.push([0, 0, 1]);
    const repaired = repairPrintMesh(mesh, 100, false);
    expect(repaired.repairs.some((message) => message.includes('degenerate'))).toBe(true);
    expect(analyzePrintMesh(repaired.mesh).degenerateTriangles).toBe(0);
  });
});

describe('M6 explicit sizing and print formats', () => {
  it('scales the longest dimension to the requested millimetres and places it on Z=0', () => {
    const soup: Array<[number, number, number]> = [
      [0, 0, 0], [2, 0, 0], [0, 1, 0],
    ];
    const transformed = transformTriangleSoupForPrint(soup, {
      ...defaultPrintPrepOptions(),
      targetMaxDimensionMm: 120,
    });
    const mesh = indexTriangleSoup(transformed.points, 120);
    const xs = mesh.vertices.map((vertex) => vertex[0]);
    const zs = mesh.vertices.map((vertex) => vertex[2]);
    expect(Math.max(...xs) - Math.min(...xs)).toBeCloseTo(120, 5);
    expect(Math.min(...zs)).toBeCloseTo(0, 8);
  });

  it('builds a standards-shaped 3MF package with millimetre units', () => {
    const archive = unzipSync(build3mfPackage(prepared(), 'Test model'));
    expect(Object.keys(archive).sort()).toEqual(['3D/3dmodel.model', '[Content_Types].xml', '_rels/.rels'].sort());
    const modelXml = strFromU8(archive['3D/3dmodel.model']);
    expect(modelXml).toContain('unit="millimeter"');
    expect(modelXml).toContain('<object id="1" type="model">');
    expect(modelXml).toContain('<triangle v1="0" v2="2" v3="1"/>');
  });

  it('writes a binary STL with one 50-byte facet record per triangle', () => {
    const bytes = buildBinaryPreparedStl(prepared());
    expect(bytes.byteLength).toBe(84 + 4 * 50);
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    expect(view.getUint32(80, true)).toBe(4);
  });
});
