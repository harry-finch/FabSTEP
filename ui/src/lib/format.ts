// Formatage des nombres (selon la langue) et conversion d'unités pour
// l'affichage. Le moteur et les fichiers travaillent toujours en millimètres.

import { i18n, intlLocale } from './i18n.svelte';

export type UnitSystem = 'mm' | 'in';

export const MM_PER_IN = 25.4;
const cache = new Map<string, Intl.NumberFormat>();

/** Nombre dans la langue courante : 16 850,5 (fr) ou 16,850.5 (en). */
export function num(value: number | null | undefined, decimals = 0): string {
  if (value == null || !Number.isFinite(value)) return '—';
  const loc = intlLocale();
  const key = `${loc}:${decimals}`;
  let f = cache.get(key);
  if (!f) {
    f = new Intl.NumberFormat(loc, { minimumFractionDigits: decimals, maximumFractionDigits: decimals });
    cache.set(key, f);
  }
  return f.format(value);
}

/** Nombre avec au plus `decimals` décimales, sans zéros inutiles. */
export function numTrim(value: number | null | undefined, decimals = 2): string {
  if (value == null || !Number.isFinite(value)) return '—';
  return new Intl.NumberFormat(intlLocale(), { maximumFractionDigits: decimals }).format(value);
}

export const lengthUnit = (u: UnitSystem) => (u === 'mm' ? 'mm' : 'in');

export function toDisplayLength(mm: number, u: UnitSystem): number {
  return u === 'mm' ? mm : mm / MM_PER_IN;
}

export function fromDisplayLength(v: number, u: UnitSystem): number {
  return u === 'mm' ? v : v * MM_PER_IN;
}

export function lengthDecimals(u: UnitSystem, mmDecimals = 2): number {
  return u === 'mm' ? mmDecimals : 3;
}

export function fmtLength(mm: number | null | undefined, u: UnitSystem, mmDecimals = 2): string {
  if (mm == null) return '—';
  return numTrim(toDisplayLength(mm, u), lengthDecimals(u, mmDecimals));
}

/** Longueur avec son unité : « 4,5 mm » / « 0.177 in ». */
export const fmtLen = (mm: number | null | undefined, u: UnitSystem, mmDecimals = 2) =>
  mm == null ? '—' : `${fmtLength(mm, u, mmDecimals)} ${lengthUnit(u)}`;

/** Aire : cm² ou in². */
export function fmtArea(mm2: number, u: UnitSystem): string {
  return u === 'mm' ? `${num(mm2 / 100, 1)} cm²` : `${num(mm2 / (MM_PER_IN * MM_PER_IN), 2)} in²`;
}

/** Pourcentage : « 12 % » en français, « 12% » en anglais. */
export const pct = (v: number, decimals = 0) => `${num(v, decimals)}${i18n.locale === 'fr' ? ' ' : ''}%`;
