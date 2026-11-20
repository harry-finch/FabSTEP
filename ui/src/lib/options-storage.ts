import type { Options } from './types.ts';
export const toleranceFields = ['weld_tolerance_mm', 'plane_tolerance_mm', 'cylinder_fit_tolerance_mm', 'max_deviation_mm'] as const;

export function exactTolerance(options: Options, field: typeof toleranceFields[number], value: number): Options {
  return { ...options, [field]: value, auto_tune: false };
}

/** One-time strategy migration. Explicit v2 strategy choices remain untouched. */
export function restoreOptions(initial:Options, current:string|null, legacy:string|null):Options {
  let saved:Partial<Options>|null=null, migrated=false;
  try { saved=current ? JSON.parse(current) : null; } catch { /* Invalid preferences use defaults. */ }
  if (!saved || typeof saved !== 'object' || Array.isArray(saved)) {
    migrated=true;
    try { saved=legacy ? JSON.parse(legacy) : null; } catch { saved=null; }
  }
  if(!saved||typeof saved!=='object'||Array.isArray(saved))return {...initial};
  const positive=(value:unknown,fallback:number)=>typeof value==='number'&&Number.isFinite(value)&&value>0?value:fallback;
  const modes=['auto','planar','cylinders','primitives','surfaces','height_patch'];
  const restored: Options = {
    input_unit:saved.input_unit==='inch'?'inch':initial.input_unit,
    reconstruction:!migrated&&modes.includes(saved.reconstruction??'')?saved.reconstruction!:'auto',
    weld_tolerance_mm:positive(saved.weld_tolerance_mm,initial.weld_tolerance_mm),
    plane_tolerance_mm:positive(saved.plane_tolerance_mm,initial.plane_tolerance_mm),
    cylinder_fit_tolerance_mm:positive(saved.cylinder_fit_tolerance_mm,initial.cylinder_fit_tolerance_mm),
    max_deviation_mm:positive(saved.max_deviation_mm,initial.max_deviation_mm),
  };
  // Older preferences recorded the numbers only. Preserve every non-default tolerance.
  restored.auto_tune = saved.auto_tune !== false && toleranceFields.every(field => restored[field] === initial[field]);
  return restored;
}
