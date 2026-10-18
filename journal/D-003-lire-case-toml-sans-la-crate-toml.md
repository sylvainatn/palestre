# D-003 — Lire `case.toml` sans la crate `toml`

| Champ       | Valeur                                                  |
|-------------|---------------------------------------------------------|
| Identifiant | D-003                                                   |
| Date        | 2026-10-18                                              |
| Auteurs     | Sylvain, Shany, Vincent                                 |
| Statut      | `proposée`                                              |
| Commits     | 52a428a                                                 |
| Clauses     | README §3 (dépendances), `corpus/FORMAT.md`, K.3, B.1   |

## 1. Contexte

Le harnais de tests doit rejouer les 65 cas publics, et les cas cachés de la soutenance, qui ont le même
format. Chaque cas décrit ses exécutions dans un `case.toml`. Or la crate `toml` n'est pas dans la liste des
dépendances autorisées du README §3, et les générateurs de parseurs sont interdits. Il faut pourtant lire ces
fichiers de façon fiable : un cas mal lu est un cas rejoué avec la mauvaise mémoire ou le mauvais budget, et la
divergence serait imputée à la machine.

## 2. Options envisagées

### Option A — Un lecteur écrit à la main, limité au sous-ensemble de `FORMAT.md`

`FORMAT.md` n'emploie que : `clé = "chaîne"`, `clé = entier`, `[[exec]]`, une table en ligne `{ 3 = 42 }`, un
tableau de tables `[{ sense = 0 }, …]`, `true`/`false`, et des commentaires `# …`. Un relevé sur les 65 cas
publics confirme qu'aucune autre construction n'apparaît.

* **Pour** : aucune dépendance à faire valider ; 300 lignes dont la moitié de tests ; chaque clé inconnue ou
  construction hors sous-ensemble est une erreur explicite, donc un cas caché inattendu se voit immédiatement.
* **Contre** : ce n'est pas du TOML complet. Un cas caché qui utiliserait une table `[exec.mem]` sur plusieurs
  lignes, une chaîne littérale `'…'` ou un entier hexadécimal serait refusé. Il faudrait alors étendre le lecteur
  le jour de la soutenance.

### Option B — Demander l'autorisation de la crate `toml`

* **Pour** : TOML complet, zéro code à maintenir.
* **Contre** : l'autorisation n'est pas acquise, et la crate `toml` tire `serde`, `toml_edit`, `winnow` — ce
  dernier est un combinateur de parseurs, ce que l'esprit de l'interdiction vise. Sans réponse de l'enseignant,
  l'équipe serait bloquée sur le harnais, qui est sur le chemin critique de la VM.

### Option C — Convertir les `case.toml` en JSON une fois pour toutes

Un script transformerait le corpus en `case.json`, lu par `serde_json`.

* **Pour** : rien à parser.
* **Contre** : le corpus est une donnée fournie, pas la nôtre ; les cas cachés arrivent en TOML le jour de la
  soutenance et il faudrait les convertir sur place, avec un outil extérieur, sans réseau. Et la conversion
  elle-même demande un lecteur TOML quelque part.

## 3. Ce que l'assistant IA a proposé

Claude Code a d'abord relevé le sous-ensemble réellement utilisé par les 65 `case.toml` publics (une commande
`sort | uniq -c` sur toutes les lignes hors métadonnées), puis a proposé et écrit l'**option A** sous la forme du
module `src/corpus.rs` :

* un `Scanner` par ligne, qui lit par `get` et jamais par indexation ; un `Value` à cinq variantes (chaîne,
  entier, booléen, table, tableau) ; un `Case` et un `Exec` typés, où `mem` est déjà dense sur 256 cellules et
  `budget` déjà borné à `1..=65535`, pour que le harnais n'ait plus rien à vérifier ;
* les quatre réponses de `FORMAT.md` (`sense`, `sense_fault`, `act`, `rand`), bien que seules `sense` et `act`
  apparaissent dans les cas publics : les cas cachés peuvent employer les deux autres ;
* une lecture **stricte** : clé inconnue, réponse à deux clés, cellule hors de `0..=255`, budget nul,
  `load_error` accompagné d'un `[[exec]]`, texte après la valeur — tout est une erreur qui nomme la ligne.
  L'assistant a argumenté qu'un harnais tolérant est pire qu'un harnais qui refuse : un cas lu de travers
  produit une divergence qu'on cherchera dans la machine ;
* la constante `MEM_SIZE = 256` placée dans `vm`, pas dans `corpus`, parce qu'elle vient de B.1 et que la
  machine en sera propriétaire ; `corpus` l'importe ;
* `description`, `category` et `clause` manquants ne sont **pas** des erreurs : ils n'influent pas sur le rejeu.

L'assistant a signalé la limite de l'option A : si un cas caché sort du sous-ensemble, le lecteur le refuse
avec un message qui nomme la ligne et la construction, et il faudra l'étendre en séance. Il a jugé ce risque
acceptable parce que `FORMAT.md` est le contrat du corpus, et que les cas publics le respectent sans exception.

**Ce que nous avons vérifié** : _à compléter — par exemple : relancer le relevé des lignes du corpus ; lire
`strip_comment` et expliquer pourquoi un `#` dans une chaîne n'est pas un commentaire ; constater que le test
d'intégration lit les 65 cas et trouve 68 exécutions, le même nombre que les `expected.json`._

### Relecture indépendante, le même jour

Une relecture automatisée lancée après l'écriture (un agent de revue de code, séparé de celui qui avait écrit le
module) a trouvé deux défauts réels dans la première version, tous deux contraires à l'objectif de lecture stricte
annoncé ci-dessus :

* le lecteur était **ligne par ligne** : un tableau `answers` écrit sur plusieurs lignes, la forme naturelle d'une
  longue liste, était refusé. Corrigé par un découpage en **lignes logiques** : une ligne dont un `[` ou un `{`
  reste ouvert se poursuit sur la suivante ;
* une **clé répétée** était acceptée, et de façon incohérente (un second `answers` s'ajoutait, un second `budget`
  écrasait). Corrigé : toute clé répétée dans une section, et toute cellule répétée dans `mem`, est une erreur.

La relecture a aussi relevé du nettoyage, appliqué dans le même commit : la conversion chaîne → `Fault` déplacée
dans `fault.rs` sous `FromStr` (le réseau en aura besoin aussi), les entiers alignés sur TOML (`+5` accepté,
`1__0` et `007` refusés), le numéro de ligne exact pour le conflit `load_error` / `[[exec]]`, l'aide `case_dirs`
partagée entre les tests d'intégration, et la CI qui lance `cargo test` sans `--all-targets` pour ne pas sauter
les exemples de documentation.

Leçon pour l'équipe : le premier jet d'un assistant est à relire comme n'importe quel code, et une seconde lecture
indépendante trouve ce que l'auteur ne voit plus. Les deux défauts auraient été invisibles sur le corpus public et
se seraient révélés le jour de la soutenance.

## 4. Ce que nous avons décidé, et pourquoi

_À compléter par l'équipe au moment d'adopter._

## 5. Ce qui nous ferait changer d'avis

* Un cas caché refusé en soutenance pour une construction TOML légitime : on étendrait le lecteur, et cette
  entrée passerait en `révisée` avec la construction ajoutée.
* L'enseignant qui autoriserait explicitement la crate `toml` : on la prendrait pour le harnais seulement, et le
  lecteur maison resterait comme solution de repli hors réseau.
* Un futur format de fichier plus riche pour nos propres cas (arène, tournois) : à ce moment-là, un vrai TOML
  ou un JSON serait plus raisonnable qu'un sous-ensemble étendu à l'infini.

## Révisions

_(aucune)_
