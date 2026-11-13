# Revue des ADR — 2026-11-13

Revue réalisée avec Codex à la demande de Shany pour préparer la validation du ticket 01. Cette note rapporte des vérifications présentes, sans attester de vérifications humaines en octobre ni d'une adoption par l'équipe.

## Vérifications exécutées

| Commande | Résultat |
|----------|----------|
| `cargo fmt --all --check` | Réussite |
| `cargo test` | Réussite : 50 tests unitaires de bibliothèque et 3 tests d'intégration ; aucun test documentaire présent |
| `cargo clippy --all-targets --all-features -- -D warnings` | Réussite, aucun avertissement |

Cargo a nécessité l'accès à son cache pour installer des dépendances manquantes, puis les vérifications ont abouti.

Le test `every_public_case_is_read_and_agrees_with_its_expected_json` vérifie notamment les comptes de 3 refus, 68 exécutions et 13 réponses scriptées. Les tests du chargeur confrontent les programmes publics aux attentes de chargement et vérifient leur encodage/décodage. Ils n'exécutent pas les instructions dans une VM.

## Limites et validation restante

* Les tests ne prouvent pas l'absence de toute panique, ni la compatibilité TOML complète.
* Les lints et recherches textuelles de la CI restent des garde-fous ; la totalité du workflow distant n'a pas été exécutée ici.
* D-001 à D-003 contiennent des propositions de décision enrichies par la revue ; leurs dates d'ouverture et leur statut `proposée` sont conservés.
* D-004 compare deux contrats `Env` avant toute implémentation. Aucun comportement de VM, d'arène ou de réseau n'est présenté comme testé.
* L'équipe doit relire les propositions, consigner son choix, ses raisons, ses noms et la date réelle de validation dans chaque ADR, puis actualiser les statuts et l'index. Le ticket Trello reste à valider après ce choix humain.
