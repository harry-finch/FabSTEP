// Langue de l'interface. Les dictionnaires vivent dans `locales/` : le
// français est la référence, l'anglais doit avoir exactement les mêmes clés
// (vérifié par TypeScript). `t('cle.sous_cle', { n: 3 })` remplace `{n}`.

import fr from './locales/fr';
import en from './locales/en';
import { storageKey } from './app';

export type Locale = 'fr' | 'en';
export const LOCALES: { id: Locale; label: string }[] = [
  { id: 'fr', label: 'Français' },
  { id: 'en', label: 'English' },
];

type Leaves<T, P extends string = ''> = {
  [K in keyof T & string]: T[K] extends string ? `${P}${K}` : Leaves<T[K], `${P}${K}.`>;
}[keyof T & string];

/** Clé de traduction valide (chemin vers un texte du dictionnaire français). */
export type Key = Leaves<typeof fr>;

const KEY = storageKey('locale.v1');
const DICTS: Record<Locale, unknown> = { fr, en };

function initial(): Locale {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === 'fr' || saved === 'en') return saved;
  } catch {
    /* stockage indisponible */
  }
  const nav = typeof navigator !== 'undefined' ? navigator.language.toLowerCase() : 'fr';
  return nav.startsWith('en') ? 'en' : 'fr';
}

class I18n {
  locale = $state<Locale>(initial());

  constructor() {
    // index.html est en « fr » : on aligne la page sur la langue retenue dès le chargement.
    if (typeof document !== 'undefined') document.documentElement.lang = this.locale;
  }

  set(l: Locale) {
    this.locale = l;
    try {
      localStorage.setItem(KEY, l);
    } catch {
      /* ignoré */
    }
    document.documentElement.lang = l;
  }
}

export const i18n = new I18n();

/** Locale `Intl` : `fr-FR` (16 850,5) ou `en-GB` (16,850.5 ; dates jj/mm/aaaa). */
export const intlLocale = () => (i18n.locale === 'fr' ? 'fr-FR' : 'en-GB');

/** « Libellé : valeur » (espace avant les deux-points en français seulement). */
export const labelled = (label: string, value: string) => (i18n.locale === 'fr' ? `${label} : ${value}` : `${label}: ${value}`);

function lookup(dict: unknown, key: string): string | undefined {
  let node = dict as Record<string, unknown> | undefined;
  for (const part of key.split('.')) node = node?.[part] as Record<string, unknown> | undefined;
  return typeof node === 'string' ? node : undefined;
}

export function t(key: Key, params?: Record<string, string | number>): string {
  const text = lookup(DICTS[i18n.locale], key) ?? lookup(fr, key) ?? key;
  return params ? text.replace(/\{(\w+)\}/g, (_, k: string) => String(params[k] ?? `{${k}}`)) : text;
}
