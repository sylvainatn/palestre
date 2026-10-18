# D-001 — Structure du projet Cargo

| Champ       | Valeur                                                                       |
|-------------|------------------------------------------------------------------------------|
| Identifiant | D-001                                                                        |
| Date        | 2026-10-18                                                                   |
| Auteurs     | Sylvain, Shany, Vincent                                                      |
| Statut      | `proposée`                                                                   |
| Commits     | _à renseigner au commit d'initialisation_                                    |
| Clauses     | README §2 (deux composants `pbc` / `exec`), §3 (contraintes), §5 (critère 1) |

## 1. Contexte

Le dépôt ne contient que le sujet : `README.md`, `SPEC-PBC-v1.md` et le corpus. Avant la première ligne de Rust,
il faut fixer la forme du projet Cargo : un ou plusieurs crates, le découpage en modules, où vivent le binaire
`palestre` et les tests du corpus, et comment les contraintes du §3 (zéro `unsafe`, zéro `panic`, zéro `async`) sont
**imposées par l'outillage** plutôt que par la relecture.

Le jury note la « séparation nette entre le format PBC, le moteur d'exécution et la couche réseau » (README §5,
critère 1) et interroge chaque ligne en soutenance. La structure doit donc être simple à expliquer, et le découpage
doit rendre les frontières visibles.

## 2. Options envisagées

### Option A — Un seul crate, une bibliothèque découpée en modules, un binaire

```
Cargo.toml
src/lib.rs        pbc, vm, arena, net (+ asm plus tard)
src/main.rs       binaire `palestre` (clap)
tests/            harnais du corpus, tests d'intégration
journal/          ADR
```

* **Pour** : un seul `Cargo.toml`, une seule commande pour tout construire, la visibilité `pub(crate)` suffit pour
  cacher les détails entre modules, la navigation en soutenance tient dans un seul arbre `src/`.
* **Contre** : la frontière entre modules n'est garantie que par la discipline (un module peut importer n'importe
  quel autre) ; tout recompile ensemble ; un `no_std` futur (bonus) demanderait de réorganiser.

### Option B — Un workspace de plusieurs crates (`pbc`, `vm`, `arena`, `net`, `cli`)

* **Pour** : les dépendances entre composants sont **déclarées** dans les `Cargo.toml` et vérifiées par le
  compilateur (`net` dépend de `vm`, jamais l'inverse) ; `vm` peut viser `no_std` seul ; compilation incrémentale
  par crate.
* **Contre** : cinq manifestes à tenir, des chemins d'import plus longs, une surface plus intimidante pour une
  équipe qui découvre Rust, et plus de choses à justifier devant le jury pour un gain qui ne se voit qu'à grande
  échelle.

### Option C — Un crate, mais les contraintes du §3 laissées à la relecture

Même structure que A, sans lints `clippy` de restriction ni étape de CI dédiée aux interdits.

* **Pour** : rien à apprendre, rien à contourner dans les tests.
* **Contre** : un `unwrap()` oublié dans le décodeur passe `cargo clippy -- -D warnings` sans bruit ; l'invariant 5
  reposerait sur la vigilance humaine, ce que le sujet rejette explicitement (« se teste par fuzzing, pas par
  relecture »).

## 3. Ce que l'assistant IA a proposé

Claude Code (Fable 5.1) a proposé l'**option A** complétée par l'outillage de l'option C en négatif, et l'a mise en
place tel quel :

* un crate `palestre`, bibliothèque + binaire, modules `pbc`, `vm`, `arena`, `net`, avec un commentaire de module
  qui cite les annexes couvertes ; `asm` ajouté plus tard si l'équipe fait le bonus ;
* `[lints.rust] unsafe_code = "forbid"` dans `Cargo.toml`, qui vaut pour tout le paquet, tests compris ;
* dans `lib.rs` et `main.rs`, `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used,
  clippy::panic, clippy::unreachable, clippy::todo, clippy::unimplemented, clippy::indexing_slicing))]` : ces lints
  sont des erreurs dans le code livré mais pas dans les tests, où le sujet tolère `unwrap` ;
* `clippy::indexing_slicing` en particulier, qui interdit `tableau[i]` et force `.get(i)` : l'assistant a signalé
  que c'est le lint le plus contraignant du lot (même `stack[sp]` avec un indice déjà vérifié est refusé) et qu'il
  est le plus facile à retirer si l'équipe le juge trop lourd ;
* une constante `SPEC_VERSION = "1.9.0"` dans `lib.rs`, seul endroit où le numéro est écrit (annexe H.2) ;
* une CI GitHub Actions : `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build`,
  `cargo test`, puis un `grep` qui échoue si `async`/`.await`/`tokio` apparaît dans `src/` ou si une crate
  interdite apparaît dans `Cargo.toml` ;
* `rust-toolchain.toml` sur `stable` avec `rustfmt` et `clippy` ;
* les dépendances autorisées ajoutées d'emblée (`clap`, `thiserror`, `serde`, `serde_json`, `sha2`, `proptest` en
  dev), même celles dont on n'a pas encore besoin, pour ne pas retoucher le manifeste à chaque jalon ;
* `missing_docs = "warn"` pour pousser à documenter chaque élément public.

L'assistant a écarté l'option B en avançant que le projet fait 30 h par personne, que la frontière « `net` ne
dépend pas de `arena` » se vérifie à l'œil dans un seul `lib.rs`, et que le bonus `no_std` reste faisable plus
tard en extrayant `vm` dans un crate le jour où il est entrepris.

**Ce que nous avons vérifié** : _à compléter — par exemple, que `cargo clippy --all-targets -- -D warnings` passe
sur le squelette, que le lint `indexing_slicing` ne bloque pas les tests, que le `grep` de la CI ne déclenche pas
sur un commentaire._

## 4. Ce que nous avons décidé, et pourquoi

_À compléter par l'équipe au moment d'adopter._

Si l'équipe suit la proposition : option A avec les lints de restriction, parce que c'est la structure la plus
simple à défendre en soutenance, que les contraintes du §3 sont vérifiées par le compilateur et la CI plutôt que par
relecture, et que rien n'empêche de passer en workspace plus tard si le bonus `no_std` l'exige.

## 5. Ce qui nous ferait changer d'avis

* Un besoin réel d'isoler `vm` pour une cible `no_std` ou WebAssembly (bonus §6) : passage en workspace, `vm` en
  crate séparé, ce qui serait une révision de cette entrée, pas une nouvelle décision.
* Une dépendance accidentelle `vm → net` ou `vm → arena` qui passerait inaperçue : signe que la discipline de
  l'option A ne suffit plus.
* Le lint `indexing_slicing` qui rendrait la boucle d'exécution illisible (`get` partout) sans attraper un seul
  bug réel : on le retirerait et on s'appuierait sur le fuzzing du chargeur, en le notant ici.
* Un temps de compilation qui ralentirait vraiment la boucle `cargo test` pendant le développement de la VM.

## Révisions

_(aucune)_
