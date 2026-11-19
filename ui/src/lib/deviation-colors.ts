import { Color } from 'three';
import type { DeviationReport } from './deviation-map.ts';

/** The preview is unindexed. Native deviations use the indexed STEP vertex order. */
export function writeDeviationColors(colors:Float32Array, triangles:number[][], report:DeviationReport, css:CSSStyleDeclaration) {
  const palette = ['low','mid','high'].map(stop => new Color(css.getPropertyValue('--view-deviation-' + stop).trim()));
  const color = new Color();
  let offset = 0;
  for (const triangle of triangles) {
    for (const vertex of triangle) {
      const fraction = Math.min(1, report.vertex_distances_mm[vertex] / report.budget_mm) * 2;
      const lower = Math.min(1, Math.floor(fraction));
      color.lerpColors(palette[lower], palette[lower+1], fraction-lower).toArray(colors, offset);
      offset += 3;
    }
  }
}
