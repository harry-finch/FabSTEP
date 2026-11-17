/** Admission hint only: native code checks the same framing and all geometry. */
export function largeStlTriangles(bytes: Uint8Array, format: 'stl' | '3mf'): number | null {
  if (format !== 'stl' || bytes.byteLength < 84 || bytes.byteLength > 64 * 1024 * 1024) return null;
  const count = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(80, true);
  return count > 100_000 && count <= 600_000 && bytes.byteLength === 84 + 50 * count ? count : null;
}
