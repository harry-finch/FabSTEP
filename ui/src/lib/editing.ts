import type { BodyProvenance, Options } from './types';

/** All values stored as preferences must be finite and strictly positive. */
export function validPreferences(value: Options): boolean {
  return [value.weld_tolerance_mm, value.plane_tolerance_mm, value.cylinder_fit_tolerance_mm, value.max_deviation_mm].every(n => Number.isFinite(n) && n > 0);
}

export function sameOptions(a: Options, b: Options): boolean {
  return (Object.keys(a) as (keyof Options)[]).every(key => Object.is(a[key], b[key]));
}

/** Return the previous committed state only for a new valid edit. */
export function commitSnapshot(draft: Options, previous: Options): Options | null {
  return validPreferences(draft) && !sameOptions(draft, previous) ? { ...previous } : null;
}

/** Translate generated labels only; names supplied by the source remain verbatim. */
export function bodyLabel(body: { id: string; name: string; provenance?: BodyProvenance }, stlLabel: string, objectLabel?: (object: number, instance: number) => string): string {
  if (body.id === 'stl' || body.id.startsWith('stl/component/')) return stlLabel;
  const p = body.provenance;
  if (p?.generated_name && p.object_id != null && p.build_item != null && objectLabel) return objectLabel(p.object_id, p.build_item + 1);
  return body.name;
}

export type NativeError = { code?: string; message: string; body?: { id: string; name: string; detail: string } | null };

/** Use structured context; the legacy message remains untouched when context is absent. */
export function errorContext(error: NativeError, stlLabel: string, resolveBody?: (id: string) => string | undefined): { detail: string; prefix: string } {
  return error.body
    ? { detail:error.body.detail, prefix:`${resolveBody?.(error.body.id) ?? bodyLabel(error.body, stlLabel)} [${error.body.id}] : ` }
    : { detail:error.message, prefix:'' };
}
