# Contrat de mission et rapport — sous-agents

## Brief transmis par l'orchestrateur

```markdown
MISSION: <identifiant court>
ROLE: explorer | implementer | reviewer | tests | docs | autre
OBJECTIF: <résultat vérifiable>
CRITÈRES D'ACCEPTATION:
- <critère observable>
CONTEXTE MINIMAL: <chemins, interfaces, contraintes>
OWNERSHIP: <fichiers/dossiers autorisés en écriture, ou READ-ONLY>
INTERDIT: <zones à ne pas toucher, actions externes>
DÉPENDANCES: <autres lots ou décisions>
EFFORT: low | medium | high | xhigh (si supporté)
VALIDATION: <tests/commandes/vérifications attendues>
STOP/ESCALATE: <quand remonter sans poursuivre>
LIVRABLE: rapport conforme au contrat ci-dessous.
```

## Rapport attendu (factuel, compact, sans log brut)

```markdown
MISSION: <identifiant>
STATUS: DONE | PARTIAL | BLOCKED | FAILED
SUMMARY: <2-5 lignes décrivant ce qui a réellement été fait>
CHANGES:
- <fichier / comportement / justification>
TESTS:
- <commande ou contrôle> — PASS | FAIL | NOT_RUN — <preuve brève>
EVIDENCE:
- <chemin, ligne, commit ou artefact réellement observé>
RISKS:
- <risque résiduel ou NONE>
OPEN_QUESTIONS:
- <question réellement bloquante ou NONE>
RECOMMENDATION: ACCEPT | REWORK | REVIEW | ESCALATE
```

## Règles de preuve

- Distinguer preuve observée, hypothèse et proposition ; ne jamais déclarer des tests PASS sans les avoir exécutés ou vérifiés via une trace crédible.
- Résumer les journaux, ne pas transférer les traces complètes sauf demande.
- Indiquer exactement ce qui n'a pas été testé et pourquoi.
- Si aucun code n'a été modifié : `CHANGES: NONE`.
- Un reviewer compare les changements aux critères d'acceptation et fournit des constats classés par gravité, avec fichier et ligne quand possible ; ne valide pas seulement le rapport du développeur.
- L'agent principal peut refuser un rapport incomplet, vérifier directement les preuves et demander une correction ciblée.
