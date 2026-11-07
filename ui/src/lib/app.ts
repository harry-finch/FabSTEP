// Identité de l'application : nom affiché et préfixe des clés de stockage
// local (`fabstep.theme.v1`, `fabstep.section.plate`…). Remplacer `fabstep`
// par l'identifiant de l'application, en minuscules, sans espace.

export const APP_ID = 'fabstep';
export const APP_NAME = 'FabSTEP';

/** Clé du stockage local, préfixée par l'application. */
export const storageKey = (key: string) => `${APP_ID}.${key}`;
