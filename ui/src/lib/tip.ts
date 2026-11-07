// Infobulle légère : `use:tip={'texte'}`. Apparaît au survol ou au focus
// clavier, après un court délai. Le texte est aussi exposé aux lecteurs d'écran.

import type { Action } from 'svelte/action';

let bubble: HTMLDivElement | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;

function ensureBubble(): HTMLDivElement {
  if (bubble) return bubble;
  bubble = document.createElement('div');
  bubble.setAttribute('role', 'tooltip');
  Object.assign(bubble.style, {
    position: 'fixed',
    zIndex: '1000',
    maxWidth: '280px',
    padding: '6px 8px',
    borderRadius: '4px',
    background: 'var(--tooltip-bg)',
    color: 'var(--tooltip-text)',
    font: '500 11.5px/1.4 var(--font)',
    boxShadow: 'var(--tooltip-shadow)',
    pointerEvents: 'none',
    opacity: '0',
    transition: 'opacity 120ms',
    whiteSpace: 'pre-line',
  });
  document.body.appendChild(bubble);
  return bubble;
}

function show(el: HTMLElement, text: string) {
  const b = ensureBubble();
  b.textContent = text;
  const r = el.getBoundingClientRect();
  const bw = Math.min(280, b.offsetWidth || 280);
  let left = r.left + r.width / 2 - bw / 2;
  left = Math.max(8, Math.min(left, window.innerWidth - bw - 8));
  b.style.left = `${left}px`;
  const below = r.bottom + 6;
  b.style.top = `${below}px`;
  requestAnimationFrame(() => {
    const h = b.offsetHeight;
    if (below + h > window.innerHeight - 8) b.style.top = `${r.top - h - 6}px`;
    b.style.opacity = '1';
  });
}

function hide() {
  clearTimeout(timer);
  if (bubble) bubble.style.opacity = '0';
}

export const tip: Action<HTMLElement, string | null | undefined> = (el, text) => {
  let current = text ?? '';
  if (current && !el.getAttribute('aria-label') && !el.textContent?.trim()) el.setAttribute('aria-label', current);
  const enter = () => {
    if (!current) return;
    clearTimeout(timer);
    timer = setTimeout(() => show(el, current), 380);
  };
  el.addEventListener('mouseenter', enter);
  el.addEventListener('focus', enter);
  el.addEventListener('mouseleave', hide);
  el.addEventListener('blur', hide);
  el.addEventListener('mousedown', hide);
  return {
    update(next) {
      current = next ?? '';
    },
    destroy() {
      hide();
      el.removeEventListener('mouseenter', enter);
      el.removeEventListener('focus', enter);
      el.removeEventListener('mouseleave', hide);
      el.removeEventListener('blur', hide);
      el.removeEventListener('mousedown', hide);
    },
  };
};
