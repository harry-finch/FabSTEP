// Figures maison : celles que la skill `hairline-create` a dessinées pour
// l'application. Chacune est un fichier `<nom>.js` écrit pour le noyau de
// Hairline (`kernel.js`) ; les deux sont servis tels quels, sans retouche,
// depuis `public/illustrations/`. `made('nom')` rend une fonction de même
// forme que celles du catalogue (`sieve`, `drawer`…), à passer à
// `Figure.svelte` : `<Figure figure={made('endmill')} label={t('figure.endmill')} />`.
//
// Ce module remplace la page d'essai de la skill (`bench.html`) : il charge
// le noyau une fois, puis la figure, et la monte comme le paquet monte les
// siennes (même boîte, mêmes options, mêmes jetons).
//
// Une figure maison peut aussi suivre des paramètres : si elle expose
// `shape(params)` à côté de `set` et `destroy`, l'option `params` lui est
// transmise à chaque changement (aperçu d'un objet en cours de saisie).

import type { Figure, HairlineOptions } from '@lucasmarkes/hairline';

type Stop = [number, number] | null;
type Engine = { set(value: number): void; destroy(): void; shape?(params: unknown): void };

/** Les options du paquet, plus les paramètres d'une figure maison qui en accepte. */
export type FigureOptions = HairlineOptions & { params?: unknown };
type Readout = { textContent: string | null };

/** Ce qu'une figure déclare en fin de fichier : `hairline({ name, means, rules, range, tour, mount })`. */
type Spec = {
  name: string;
  means: string;
  range: [number, number, number];
  tour?: Stop[] | null;
  mount(host: { stage: HTMLElement; svg: SVGSVGElement; read: Readout }, value: number): Engine;
};

/** La part du noyau dont ce module a besoin. */
type Kernel = {
  inject(root: Document | ShadowRoot): void;
  mk(tag: string, attrs: Record<string, string>, parent: Element): Element;
  tour(stage: HTMLElement, stops: Stop[]): { stop(): void };
  LAP: Stop[];
};

declare global {
  interface Window {
    HL?: Kernel;
    hairline?: (spec: Spec) => void;
  }
}

const BASE = `${import.meta.env.BASE_URL}illustrations/`;
const specs = new Map<string, Spec>();
const scripts = new Map<string, Promise<void>>();

function script(src: string, module: boolean): Promise<void> {
  let loaded = scripts.get(src);
  if (!loaded) {
    loaded = new Promise((resolve, reject) => {
      const el = document.createElement('script');
      if (module) el.type = 'module';
      el.src = src;
      el.onload = () => resolve();
      el.onerror = () => reject(new Error(`Illustration introuvable : ${src}`));
      document.head.appendChild(el);
    });
    scripts.set(src, loaded);
  }
  return loaded;
}

async function load(name: string): Promise<{ HL: Kernel; spec: Spec }> {
  // Chaque figure se déclare en appelant `hairline({ … })` : on la range par nom.
  window.hairline ??= (spec) => specs.set(spec.name, spec);
  await script(`${BASE}kernel.js`, false);
  await script(`${BASE}${name}.js`, true);
  const spec = specs.get(name);
  if (!window.HL || !spec) throw new Error(`Illustration « ${name} » : le fichier ne déclare pas de figure de ce nom.`);
  return { HL: window.HL, spec };
}

/** Le nombre propre à la figure pour une intensité de 0 à 1 : deux droites qui se rejoignent à 0,5, comme dans le paquet. */
function parameter([lo, mid, hi]: Spec['range'], intensity: unknown): number {
  const i = typeof intensity === 'number' && Number.isFinite(intensity) ? Math.min(1, Math.max(0, intensity)) : 0.5;
  const v = i <= 0.5 ? lo + (i / 0.5) * (mid - lo) : mid + ((i - 0.5) / 0.5) * (hi - mid);
  return Math.round(v * 1000) / 1000;
}

/** Une figure maison, par son nom (celui de `public/illustrations/<nom>.js`). */
export function made(name: string) {
  return (el: HTMLElement, options: FigureOptions = {}): Figure => {
    const opts: FigureOptions = { ...options };
    let dead = false;
    let apply: (() => void) | null = null;
    let undo: (() => void) | null = null;

    load(name)
      .then(({ HL, spec }) => {
        if (dead) return;
        HL.inject(document);
        el.setAttribute('data-hairline', spec.name);
        el.setAttribute('role', 'img');
        const svg = HL.mk('svg', { viewBox: '0 0 400 320', 'aria-hidden': 'true' }, el) as SVGSVGElement;

        // La légende de la figure (« rest », « 03 »…) : jamais affichée dans
        // l'application, transmise à `onRead` quand elle change.
        let text: string | null = null;
        const read: Readout = {
          get textContent() {
            return text;
          },
          set textContent(value) {
            const next = value ?? '';
            if (next === text) return;
            text = next;
            opts.onRead?.(next);
          },
        };

        // Le parcours s'inscrit avant la figure : il bouge, puis elle dessine.
        let value = parameter(spec.range, opts.intensity);
        let playing = opts.play === true ? HL.tour(el, spec.tour ?? HL.LAP) : null;
        let engine: Engine;
        try {
          engine = spec.mount({ stage: el, svg, read }, value);
        } catch (err) {
          playing?.stop();
          svg.remove();
          throw err;
        }
        if (text === null) read.textContent = 'rest';
        let shaped: string | undefined;

        apply = () => {
          el.setAttribute('aria-label', typeof opts.label === 'string' ? opts.label : spec.means);
          const v = parameter(spec.range, opts.intensity);
          if (v !== value) {
            value = v;
            engine.set(v);
          }
          // Les paramètres ne repartent vers la figure que s'ils ont changé.
          const params = opts.params === undefined ? undefined : JSON.stringify(opts.params);
          if (params !== shaped) {
            shaped = params;
            if (opts.params !== undefined) engine.shape?.(opts.params);
          }
          const play = opts.play === true;
          if (play !== !!playing) {
            playing?.stop();
            playing = play ? HL.tour(el, spec.tour ?? HL.LAP) : null;
          }
        };
        undo = () => {
          playing?.stop();
          engine.destroy();
          svg.remove();
          for (const attr of ['data-hairline', 'role', 'aria-label']) el.removeAttribute(attr);
        };
        apply();
      })
      .catch((err) => console.error(err));

    return {
      update(next) {
        if (dead || !next) return;
        Object.assign(opts, next);
        apply?.();
      },
      destroy() {
        if (dead) return;
        dead = true;
        undo?.();
      },
    };
  };
}
