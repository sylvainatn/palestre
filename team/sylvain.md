# Sylvain — `pbc` et harnais de corpus

## Fait

- [x] Chargeur `&[u8]` → `Program`, erreurs A.2 (`src/pbc/load.rs`)
- [x] Décodeur offset → instruction + taille (`src/pbc/decode.rs`)
- [x] Chargement des 65 programmes du corpus (`tests/corpus_load.rs`)
- [x] Lecteur des `case.toml` sans la crate `toml` (`src/corpus.rs`, D-003)

## À faire

- [ ] Lecture des `expected.json` avec `serde_json` (`src/corpus.rs`)
- [ ] Harnais complet : exécuter chaque cas sur la machine et comparer avec `expected.json`, dans l'ordre
      rappels → `gas_used` → `fault` → `act` → `trace` → `mem` (la `vm` de Shany est sur `master`, c'est débloqué)
- [ ] Fuzzing du chargeur : aucun octet ne doit le faire paniquer (invariant README §1, bonus avec ADR)
- [ ] Relire et faire passer D-002 et D-003 de `proposée` à `adoptée` avec l'équipe

## Relectures

- Relire `src/vm.rs` (Shany) côté décodage : tailles d'instruction, sauts relatifs B.4
