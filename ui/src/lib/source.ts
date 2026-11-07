/** Tauri's raw body must be the Uint8Array itself, including its view bounds. */
export type SourceBridge = { invoke: <T>(command: string, args: Record<string, unknown> | Uint8Array, options?: { headers: Record<string, string> }) => Promise<T> };
export async function uploadSource(bridge: SourceBridge, bytes: Uint8Array, sourceId: number, format: 'stl' | '3mf'): Promise<void> {
  if (bytes.byteLength > 64 * 1024 * 1024) throw { code:'input.too_large', message:'File exceeds 64 MiB.' };
  await bridge.invoke<void>('upload_source', bytes, { headers: { 'x-fabstep-source-id':String(sourceId), 'x-fabstep-format':format } });
}
