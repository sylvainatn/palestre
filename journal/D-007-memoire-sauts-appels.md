# D-007 — Mémoire, sauts et appels

| Champ | Valeur |
|-------|--------|
| Identifiant | D-007 |
| Date | 2026-11-15 |
| Auteurs | Proposition Codex à la demande de Shany ; revue de l'équipe en attente |
| Statut | `proposée` |
| Commits | Aucun |
| Clauses | B.3–B.4 ; ticket 05 |

## 1. Contexte

Le ticket 05 complète les instructions intégrées dans Machine par les accès mémoire et le contrôle d'exécution. Le cycle possède déjà pc, gaz et pile d'appels fixe.

## 2. Options envisagées

- Étendre execute_core pour recevoir tout State et rendre Control : un seul routage, mais modification du bloc de calcul existant.
- Ajouter une fonction privée pour mémoire et contrôle : conserve les calculs validés, au prix d'une seconde étape de routage.

## 3. Ce que l'assistant IA a proposé

Codex propose et implémente la seconde option, dans la continuité de D-006. La fonction injectée ne reçoit plus que les instructions environnementales. Les signatures publiques restent identiques.

Les accès utilisent get/get_mut après conversion vérifiée pour les adresses dépilées. STOREX dépile l'adresse puis la valeur. Les sauts relatifs utilisent l'adresse suivante et une addition signée vérifiée ; les sauts absolus convertissent la cible sans troncature. Une fonction commune vérifie l'octet cible, y compris dans un opérande. Un saut conditionnel non pris ne valide pas sa cible.

Les appels valident leur cible puis empilent l'adresse suivante. RETURN dépile cette adresse sans exiger JUMPDEST ; une adresse égale à code_len produira PcOutOfRange au décodage suivant. La pile d'appels reste limitée à seize entrées et le gaz est prélevé avant toute exécution, même fautive. Aucune allocation n'est ajoutée dans le tour.

## 4. Ce que nous avons décidé et pourquoi

Brouillon implémenté dans le périmètre demandé ; adoption humaine en attente. Les tests du cycle utilisent désormais les vrais accès mémoire et sauts. Les tests spécifiques couvrent les bornes mémoire, les deux sens des sauts, les conditions non prises, les cibles absolues invalides, les destinations dans un opérande, les retours et les fautes de pile d'appels.

## 5. Ce qui nous ferait changer d'avis

L'intégration d'Env justifie une sémantique centralisée : réunir alors les fonctions privées sans modifier le cycle normatif ni les résultats observables.
