// Identité de l'application : nom affiché et préfixe des clés de stockage
// local (`__app__.theme.v1`, `__app__.section.plate`…). Remplacer `__app__`
// par l'identifiant de l'application, en minuscules, sans espace.

export const APP_ID = '__app__';
export const APP_NAME = '__App__';

/** Clé du stockage local, préfixée par l'application. */
export const storageKey = (key: string) => `${APP_ID}.${key}`;
