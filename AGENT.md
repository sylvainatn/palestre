# AGENT.md — Consignes pour tout assistant travaillant sur Palestre

Ce fichier s'adresse à tout agent IA (Claude Code, Copilot, Cursor, etc.) qui intervient dans ce dépôt.
Il résume ce qui est non négociable. En cas de doute, la spécification fait foi.

## 1. Le projet en trois lignes

Palestre est un projet scolaire ESGI en Rust : une **machine virtuelle à pile déterministe** qui exécute le bytecode
`PBC v1`, un **chargeur** de ce format binaire, et un **serveur d'exécution distante** (annexe K) qui délègue
`SENSE`, `ACT` et `RAND` au client par rappels. Tous les groupes implémentent la même spec : le résultat, le gaz
consommé, la faute et la trace doivent être identiques entre implémentations.

## 2. Sources de vérité, dans l'ordre

1. `SPEC-PBC-v1.md` — normative, version `1.9.0`. Elle l'emporte sur tout le reste.
2. `README.md` — le sujet : livrables, barème, contraintes techniques, journal d'ADR.
3. `corpus/` — 65 cas de test (`edge/` et `nominal/`), format décrit dans `corpus/FORMAT.md`, index dans
   `corpus/INDEX.md`. Chaque cas cite la clause de la spec qu'il éprouve : la lire avant de déboguer.

Ne jamais « corriger » le corpus ou la spec pour faire passer un test. Si un cas semble contredire la spec, le
signaler explicitement à l'équipe.

## 3. Règles de code non négociables (README §3)

* Rust édition 2021 ou 2024, `std` uniquement pour la concurrence.
* `#![forbid(unsafe_code)]` sur toutes les crates. Aucune exception.
* **Pas d'`async`/`await`**, pas de `tokio`, pas de `futures`.
* **Pas de `unwrap()`, `expect()`, `panic!()`, `unreachable!()`, indexation non vérifiée** dans le chargeur, le
  décodeur, la VM et la couche réseau. Tolérés uniquement dans les tests. Toute erreur est un `Result` typé.
* **Aucune allocation dans la boucle d'exécution d'un tour.** Pile (64), pile d'appels (16), mémoire (256) sont des
  tableaux de taille fixe ; le tampon de trace est réservé avant le tour, à hauteur du budget.
* Arithmétique **enveloppante** partout : `wrapping_add`, `wrapping_mul`, `wrapping_div`, `wrapping_rem`,
  `wrapping_neg`. `i64::MIN / -1` avec `/` panique en Rust : interdit.
* `cargo clippy --all-targets -- -D warnings` et `cargo fmt --check` doivent passer avant tout commit.
* **Dépendances autorisées** : `sha2`, `thiserror`, `clap`, `serde`, `serde_json`, `rand`, `proptest`, `criterion`,
  `arbitrary`, `cargo-fuzz`. Toute autre crate doit être validée par l'enseignant **avant** d'être ajoutée.
  Ne jamais en ajouter une silencieusement ; proposer l'alternative « à la main » et laisser l'équipe trancher.
* **Interdits** : VM ou interpréteurs tiers (`wasmi`, `wasmtime`, `rhai`, `revm`, `mlua`…), générateurs de parseurs
  (`pest`, `nom`, `lalrpop`, `chumsky`). Tout parseur (assembleur, TOML du corpus, JSON hors `serde_json`) s'écrit
  à la main.
* `serde`/`serde_json` servent aux trames réseau et aux fichiers de test, jamais dans la boucle de la VM.

## 4. Architecture cible

Une seule VM, plusieurs environnements. La VM ne sait pas qui répond à `SENSE`, `ACT`, `RAND` : elle passe par un
trait `Env`. L'arène locale, le serveur réseau et le harnais de test l'implémentent chacun.

| Module  | Rôle                                                                                   |
|---------|----------------------------------------------------------------------------------------|
| `fault` | l'`enum Fault` close à 16 variantes (B.5), partagée par `pbc`, `vm`, `net` (D-002)     |
| `pbc`   | chargeur (`&[u8]` → `Program`), décodeur (offset → instruction + taille), erreurs A.2   |
| `vm`    | `Machine`, `Fault` (16 variantes closes, B.5), cycle B.2, table de gaz, trait `Env`     |
| `arena` | grille, agents, tours E.2 (instantané / restauration), capteurs E.3, actions E.4         |
| `net`   | cadrage K.2, machine d'états K.4 par connexion, JSON K.3, `palestre exec --listen`      |
| `asm`   | assembleur / désassembleur textuels (outillage, bonus) avec `asm(disasm(p)) == p`       |
| `corpus`| lecture des `case.toml` (parseur TOML minimal à la main, D-003) et des `expected.json`   |
| `cli`   | binaire `palestre` : `exec`, `run`, `match`, `asm`, `disasm`, `replay`                  |

Deux choses existent à **un seul endroit** du code, parce qu'une version `2.0.0` surprise sera révélée en soutenance :

* le numéro de version de la spec (`"1.9.0"`), repris dans `READY` ;
* la table de gaz, une fonction `fn cost(op) -> u16` et rien d'autre.

## 5. Pièges de la spec à ne jamais réinventer

* Gaz prélevé **avant** exécution (B.2). Pile pleine sans gaz ⇒ `OutOfGas`, pas `StackOverflow`. Une instruction
  refusée faute de gaz n'entre pas dans `gas_used`.
* Saut relatif à **l'instruction suivante** : `cible = pc + taille + d` (B.4). La cible doit tomber dans
  `[0, code_len)` **et** l'octet visé doit valoir `0x43`, même au milieu d'un opérande. `JMPX`/`CALLX` dépilent
  une cible **absolue**. `RETURN` n'exige pas de `JUMPDEST`.
* Notation de pile `a b -- r` : `b` est au sommet. `SHL`/`SHR`/`SAR` lisent `a n --`, `n` au sommet.
* `SHL`/`SHR` logiques sur `u64`, `SAR` arithmétique. Compte hors `0..=63` : `0` pour `SHL`/`SHR`, remplissage de
  signe pour `SAR`. Jamais de faute.
* `NOT` est logique (0/1), `BNOT` est le complément à un.
* `GAS` empile le reste **après** son propre coût : 998 à pc 0 sous budget 1000.
* `ACT` dépile l'argument puis empile 0/1. Échec d'action ≠ faute. Second `ACT` du tour ⇒ `AlreadyActed`,
  levé par la machine sans rappel. `BadAction` ne consigne aucune action.
* La VM **ne restaure pas** la mémoire sur faute : `RESULT.mem` garde les écritures antérieures. La restauration
  de E.2 est le travail de l'arène.
* `BadHeader` est une erreur de chargement, jamais une faute de tour.
* Tous les entiers multi-octets sont **big-endian**, fichier et réseau.

## 6. Commandes de travail

```bash
cargo build
cargo test                                    # unitaires + harnais corpus/
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run -- exec --listen 127.0.0.1:9000     # serveur annexe K (obligatoire)
```

Les tests d'intégration vivent dans `tests/` : `corpus_load.rs` (chargeur et décodeur contre les 65 programmes),
`corpus_cases.rs` (lecture des `case.toml`), et `common/mod.rs` pour les aides partagées. Le harnais complet,
qui exécutera chaque cas et comparera avec `expected.json`, viendra avec la machine ; il comparera **dans cet
ordre** : rappels, `gas_used`, `fault`, `act`, `trace`, puis `mem`. Corriger le premier champ faux avant de
regarder les suivants.

## 7. Journal de décisions (ADR) — obligatoire

Toute décision d'architecture va dans `journal/D-NNN-titre.md`, **avant ou avec** le code qu'elle motive, jamais
après coup. En-tête : identifiant, date, auteurs, statut (`proposée` / `adoptée` / `révisée` / `abandonnée`),
commits. Cinq sections : Contexte ; Options envisagées (au moins deux) ; **Ce que l'assistant IA a proposé**
(retranscrit fidèlement, y compris quand c'était faux) ; Ce que nous avons décidé et pourquoi ; Ce qui nous ferait
changer d'avis.

Conséquences pour un agent :

* quand on te demande de concevoir quelque chose, **propose au moins deux options** avec leurs compromis, et
  rappelle qu'une entrée de journal doit être ouverte ;
* **tu ne décides pas** : tu rédiges un brouillon, l'équipe tranche et signe ;
* ne jamais antidater, regrouper ou rédiger des ADR rétrospectivement ;
* chaque bonus revendiqué exige son propre ADR, sinon il ne compte pas.

## 8. Git et soutenance

* Commits nominatifs, un auteur humain par commit. Un agent ne commite ni ne pousse sans demande explicite.
* Les étudiants doivent **défendre chaque ligne** au clavier, sans réseau, en soutenance. Préférer toujours la
  construction simple et explicable à la construction sophistiquée. Expliquer ce que tu écris : un commentaire
  court sur le *pourquoi* vaut mieux qu'une abstraction élégante que personne ne saura modifier en direct.
* Pas de macro procédurale maison, pas de généricité gratuite, pas de trait objet là où un `enum` suffit.
* Documentation, ADR et commentaires en **français** ; identifiants de code en **anglais** ; mnémoniques et
  noms de fautes tels que la spec les écrit (`OutOfGas`, `JUMPDEST`…).

## 9. État du dépôt

Depuis le commit `52a428a` (2026-10-18), le projet Cargo existe avec la CI, et ces modules sont écrits et testés :
`fault` (les seize fautes), `pbc` (chargeur et décodeur, vérifiés contre les 65 programmes du corpus), `corpus`
(lecture des `case.toml`). `vm`, `arena` et `net` ne contiennent que leur documentation. Le journal compte trois
entrées (D-001 à D-003), en statut `proposée` tant que l'équipe n'a pas rempli leur section 4.

Prochaine décision à ouvrir : D-004, la signature du trait `Env` par lequel la machine demande `SENSE`, `ACT` et
`RAND` à son environnement. Elle précède l'écriture de `vm`.
