# D-006 — Instructions pures de la VM

| Champ | Valeur |
|-------|--------|
| Identifiant | D-006 |
| Date | 2026-11-15 |
| Auteurs | Proposition Codex à la demande de Shany ; revue de l'équipe en attente |
| Statut | `proposée` |
| Commits | Aucun |
| Clauses | B.3, B.6 ; ticket 04 |

## 1. Contexte

D-005 fournit un cycle et une fonction injectée pour les instructions restantes. Le ticket 04 ajoute constantes, pile, arithmétique, comparaisons et opérations binaires sans engager le contrat Env.

## 2. Options envisagées

- Ajouter une fonction publique à brancher dans chaque exécuteur : conserve strictement le routage initial, mais impose à chaque appelant de composer les mêmes instructions.
- Intégrer ces instructions dans le cycle, via une fonction privée : garantit leur sémantique pour chaque Machine, tout en conservant la fonction injectée pour mémoire, sauts et rappels.

## 3. Ce que l'assistant IA a proposé

Codex propose et implémente la seconde option. execute_core renvoie None pour les instructions déléguées, et Some(Result) pour celles du ticket. Le cycle prélève toujours le gaz avant cette fonction. Les tableaux et opérations de pile existants sont réutilisés ; aucune allocation n'est ajoutée pendant le tour.

Les opérandes se dépilent du sommet vers le fond. Les opérations arithmétiques utilisent les méthodes wrapping après vérification du diviseur. Le compte des décalages est vérifié avant toute opération Rust ; SHL et SHR réinterprètent la valeur en u64, SAR conserve le signe. NOT produit 0 ou 1, BNOT complémente les bits.

Les tests du cycle sont adaptés : les instructions intégrées ne passent plus par la fonction injectée. Des tests de sémantique exécutent de vrais programmes et vérifient pile, fautes et limites.

## 4. Ce que nous avons décidé et pourquoi

Brouillon implémenté dans le périmètre demandé ; adoption humaine en attente. La proposition étend D-005 sans modifier la signature de Machine::run. Les fonctions injectées existantes ne reçoivent désormais plus les instructions du ticket 04.

## 5. Ce qui nous ferait changer d'avis

La composition des prochaines instructions rend ce routage difficile à maintenir : réévaluer une sémantique complète centralisée lors de l'intégration d'Env, sans changer les règles de gaz ni les résultats observables.
