# D-005 — État de la VM et cycle d'exécution

| Champ | Valeur |
|-------|--------|
| Identifiant | D-005 |
| Date | 2026-11-13 |
| Auteurs | Implémentation proposée par Codex à la demande de Shany ; revue de l'équipe en attente |
| Statut | `proposée` |
| Commits | Aucun commit à ce stade |
| Clauses | B.1–B.3, B.7, D, E.2, K.6 ; README §3 |

## 1. Contexte

Le ticket 02 demande les ressources fixes, le résultat typé, la remise à zéro du transitoire et le cycle normatif. Le décodeur couvre déjà B.2 étapes 1 à 3. Le contrat Env de D-004 reste proposé : ce ticket ne doit pas imposer son adoption ni prétendre livrer toutes les instructions.

## 2. Options envisagées

### Option A — Cycle et sémantique de toutes les instructions dans une seule modification

Livre immédiatement un interpréteur complet, mais dépasse le ticket et engage le contrat des rappels avant sa revue.

### Option B — Cycle avec fonction d'exécution injectée

La machine possède les ressources et contrôle décodage, prélèvement, arrêt et avance du pc. Une fonction reçoit l'état, l'instruction et son adresse ; elle rend une avance normale, une cible validée, une faute ou une erreur technique. La sémantique complète pourra être branchée ensuite. L'appelant doit fournir une fonction conforme et sans allocation ; ce contrat ne peut pas être garanti pour du code extérieur arbitraire.

## 3. Ce que l'assistant IA a proposé

Codex a implémenté l'option B dans src/vm.rs. Les piles sont des tableaux avec longueur occupée. La mémoire reste dans la machine ; les piles, pc, gaz, trace et action sont réinitialisés pour chaque run. Le résultat emprunte mémoire et trace, évitant une allocation de sortie. Une faute conserve les écritures antérieures ; leur restauration appartient à l'arène.

HALT, JUMPDEST, GAS et TRACE sont directement traités par le cycle. Les autres instructions passent par la fonction injectée, sans comportement de remplacement silencieux. La table de coûts exhaustive est centralisée dans gas_cost. Le budget NonZeroU16 représente exactement 1..=65535.

La trace réserve 65535 i64 à la construction (environ 512 Kio par machine). Ce compromis évite toute réallocation lorsque le budget augmente entre deux tours. TRACE coûte 1 : le nombre d'ajouts ne dépasse jamais la capacité. La vérification de stabilité du pointeur et de la capacité complète la lecture du code ; elle n'est pas un instrument général de comptage des allocations.

Validation effectuée : cargo test (62 tests réussis, dont neuf nouveaux tests VM) et cargo clippy --all-targets -- -D warnings. Les nouveaux tests couvrent l'ordre des fautes, les coûts sur refus et faute, HALT sans gaz restant, les limites des piles, les sauts, GAS après prélèvement, TRACE, la conservation des écritures, la réinitialisation et l'interruption technique sans résultat.

## 4. Ce que nous avons décidé, et pourquoi

Implémentation réalisée sur demande de mise en place du ticket ; choix technique proposé par Codex, à relire par l'équipe. Aucune adoption humaine n'est consignée. D-004 reste inchangée : les rappels SENSE/ACT/RAND et les contrôles d'actions ne sont pas encore implémentés. La machine réinitialise et restitue l'action ; la fonction d'exécution devra la renseigner et contrôler AlreadyActed.

## 5. Ce qui nous ferait changer d'avis

L'implémentation des instructions montre que la fonction injectée complique les accès ou la gestion des rappels : intégrer alors la sémantique dans Machine en conservant les tests du cycle. Un grand nombre de machines simultanées rend la réservation maximale de la trace trop coûteuse : préparer plutôt une capacité par budget avant chaque tour.

## Révisions

_(aucune)_
