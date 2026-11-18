import type { Options } from './types.ts';
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
  return {
    input_unit:saved.input_unit==='inch'?'inch':initial.input_unit,
    reconstruction:!migrated&&modes.includes(saved.reconstruction??'')?saved.reconstruction!:'auto',
    weld_tolerance_mm:positive(saved.weld_tolerance_mm,initial.weld_tolerance_mm),
    plane_tolerance_mm:positive(saved.plane_tolerance_mm,initial.plane_tolerance_mm),
    cylinder_fit_tolerance_mm:positive(saved.cylinder_fit_tolerance_mm,initial.cylinder_fit_tolerance_mm),
    max_deviation_mm:positive(saved.max_deviation_mm,initial.max_deviation_mm),
  };
}
