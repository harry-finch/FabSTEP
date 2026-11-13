// English dictionary: exactly the same keys as fr.ts (checked by TypeScript).

import type fr from './fr';

type Shape<T> = { [K in keyof T]: T[K] extends string ? string : Shape<T[K]> };

const en: Shape<typeof fr> = {
  app: {
    tagline: 'Short application subtitle',
    units: 'Units',
    inches: 'inches',
    language: 'Language',
    theme: 'Theme',
    themes: { system: 'System theme', light: 'Light theme', dark: 'Dark theme' },
    help: 'Help',
  },
  common: {
    close: 'Close',
    cancel: 'Cancel',
    apply: 'Apply',
  },
  file: {
    menu: 'File',
    new: 'New',
    open: 'Open…',
    save: 'Save',
    project: 'Project',
    name: 'Project name',
    nameHint: 'Project name, used for exported file names.',
    untitled: 'Untitled',
  },
  export: {
    button: 'Export',
    menu: 'Export',
  },
  status: {
    ready: 'Engine ready',
    loading: 'Loading engine…',
    failed: 'Engine unavailable',
    engine: 'Engine {v}',
  },
  alerts: {
    info: '{n} note',
    infos: '{n} notes',
  },
  help: {
    title: 'Help',
    shortcuts: 'Shortcuts',
    keys: {
      help: 'Open help',
      undo: 'Undo / redo',
    },
  },
};

export default en;
