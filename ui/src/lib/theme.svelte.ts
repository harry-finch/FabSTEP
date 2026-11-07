// Thème de l'interface : sombre (référence), clair, ou celui du système.
// Le choix est mémorisé sur le poste ; `index.html` l'applique avant le
// premier affichage (pas de flash), ce module le tient à jour ensuite.

import { storageKey } from './app';

export type ThemeChoice = 'system' | 'light' | 'dark';
export const THEMES: ThemeChoice[] = ['system', 'light', 'dark'];

const KEY = storageKey('theme.v1');

function read(): ThemeChoice {
  try {
    const v = localStorage.getItem(KEY);
    if (v === 'system' || v === 'light' || v === 'dark') return v;
  } catch {
    /* stockage indisponible */
  }
  return 'system';
}

const systemLight = () =>
  typeof matchMedia !== 'undefined' && matchMedia('(prefers-color-scheme: light)').matches;

class Theme {
  choice = $state<ThemeChoice>(read());
  /** Thème réellement affiché. */
  applied = $derived<'light' | 'dark'>(this.#resolve());
  #system = $state(systemLight());

  constructor() {
    if (typeof matchMedia !== 'undefined') {
      matchMedia('(prefers-color-scheme: light)').addEventListener('change', (e) => {
        this.#system = e.matches;
        this.#apply();
      });
    }
    this.#apply();
  }

  #resolve(): 'light' | 'dark' {
    if (this.choice === 'system') return this.#system ? 'light' : 'dark';
    return this.choice;
  }

  #apply() {
    if (typeof document !== 'undefined') document.documentElement.dataset.theme = this.#resolve();
  }

  set(choice: ThemeChoice) {
    this.choice = choice;
    try {
      localStorage.setItem(KEY, choice);
    } catch {
      /* ignoré */
    }
    this.#apply();
  }
}

export const theme = new Theme();
