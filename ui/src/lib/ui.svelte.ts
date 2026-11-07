// État de l'interface commun à toutes les applications : unités d'affichage,
// aide, messages passagers. L'état métier vit dans le store de l'application.

import { storageKey } from './app';
import type { UnitSystem } from './format';

const UNITS_KEY = storageKey('units.v1');

function readUnits(): UnitSystem {
  try {
    return localStorage.getItem(UNITS_KEY) === 'in' ? 'in' : 'mm';
  } catch {
    return 'mm';
  }
}

export interface Toast {
  id: number;
  text: string;
}

class Ui {
  #units = $state<UnitSystem>(readUnits());
  help = $state(false);
  toasts = $state<Toast[]>([]);
  #next = 0;

  get units() {
    return this.#units;
  }

  set units(u: UnitSystem) {
    this.#units = u;
    try {
      localStorage.setItem(UNITS_KEY, u);
    } catch {
      /* ignoré */
    }
  }

  /** Message passager en bas de l'écran (confirmation d'une action). */
  toast(text: string, ms = 2600) {
    const id = ++this.#next;
    this.toasts = [...this.toasts, { id, text }];
    setTimeout(() => (this.toasts = this.toasts.filter((x) => x.id !== id)), ms);
  }
}

export const ui = new Ui();
