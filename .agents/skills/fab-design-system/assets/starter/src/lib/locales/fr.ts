// Dictionnaire de référence (français). Toute clé ajoutée ici doit l'être
// aussi dans en.ts : TypeScript le vérifie.
// Typographie : espace insécable avant « : ; ! ? % », guillemets « », apostrophe ’.

const fr = {
  app: {
    tagline: 'Sous-titre court de l’application',
    units: 'Unités',
    inches: 'pouces',
    language: 'Langue',
    theme: 'Thème',
    themes: { system: 'Thème du système', light: 'Thème clair', dark: 'Thème sombre' },
    help: 'Aide',
  },
  common: {
    close: 'Fermer',
    cancel: 'Annuler',
    apply: 'Appliquer',
  },
  file: {
    menu: 'Fichier',
    new: 'Nouveau',
    open: 'Ouvrir…',
    save: 'Enregistrer',
    project: 'Projet',
    name: 'Nom du projet',
    nameHint: 'Nom du projet, repris dans le nom des fichiers exportés.',
    untitled: 'Sans titre',
  },
  export: {
    button: 'Exporter',
    menu: 'Exporter',
  },
  status: {
    ready: 'Moteur prêt',
    loading: 'Chargement du moteur…',
    failed: 'Moteur indisponible',
    engine: 'Moteur {v}',
  },
  alerts: {
    info: '{n} remarque',
    infos: '{n} remarques',
  },
  help: {
    title: 'Aide',
    shortcuts: 'Raccourcis',
    keys: {
      help: 'Ouvrir l’aide',
      undo: 'Annuler / rétablir',
    },
  },
};

export default fr;
