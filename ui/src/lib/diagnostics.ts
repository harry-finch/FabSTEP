import type { Diagnostic } from './types.ts';

// These known remarks explain the explicitly chosen reconstruction and settings.
// Geometry changes and unknown codes remain visible, regardless of severity.
const technicalRemarks = new Set([
  'precision.custom_tolerances',
  'recognition.circular_intent_assumed',
  'recognition.conic_intent_assumed',
  'recognition.rounded_intent_assumed',
  'recognition.rounded_box_intent_assumed',
]);
const routineInfo = new Set([
  'auto.faceted_base',
  'auto.planar_regions_split',
  'auto.regions_faceted',
  'guidance.auto_applied',
  'guidance.torus_applied',
  'validation.native_required',
  'recognition.rounded_box_coverage',
  'recognition.rounded_coverage',
  'recognition.revolved_coverage',
  'recognition.mixed_coverage',
  'recognition.bound_scope',
  'document.separate_bodies',
  'document.product_names_adapted',
  'guidance.cylinder_applied',
  'guidance.cone_applied',
  'guidance.plane_applied',
  'guidance.plane_verified',
  'guidance.complete_cylinder_verified',
  'guidance.complete_cone_verified',
]);

export function visibleDiagnostic(diagnostic: Diagnostic): boolean {
  if (diagnostic.severity !== 'info' && diagnostic.severity !== 'warning') return true;
  return !technicalRemarks.has(diagnostic.code)
    && !(diagnostic.severity === 'info' && routineInfo.has(diagnostic.code));
}
