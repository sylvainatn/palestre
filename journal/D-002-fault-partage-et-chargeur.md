# D-002 — Où vit `Fault`, et la forme du chargeur

| Champ       | Valeur                                                      |
|-------------|-------------------------------------------------------------|
| Identifiant | D-002                                                       |
| Date        | 2026-10-18                                                  |
| Auteurs     | Sylvain, Shany, Vincent                                     |
| Statut      | `proposée`                                                  |
| Commits     | 52a428a                                                     |
| Clauses     | A.1, A.2, A.3, B.2 (étapes 1 à 3), B.5, K.2 (borne de trame) |

## 1. Contexte

Le chargeur est le premier module écrit. Deux questions se posent avant lui.

**Où déclarer `Fault` ?** B.5 donne une liste close de seize fautes. `BadHeader` est levée par le chargeur,
`BadOpcode`/`Truncated`/`PcOutOfRange` par le décodeur, les douze autres par la machine. La couche réseau les
sérialise toutes. Si `Fault` vit dans `vm`, alors `pbc` dépend de `vm` alors que c'est `vm` qui consomme un
`Program` : la dépendance tourne en rond.

**Quelle forme pour `Program` ?** Le fichier `.pbc` est lu une fois, puis exécuté autant de fois qu'il y a
d'`EXEC` sur la session (K.4). Il faut décider si `Program` possède ses octets ou emprunte le tampon reçu, et
comment garantir « aucune allocation non bornée » (A.2) sur une entrée hostile.

## 2. Options envisagées

### Où vit `Fault`

* **A — dans `vm`**, `pbc` importe `vm::Fault`. Simple, mais la dépendance `pbc → vm → pbc` est un cycle
  logique, même si Rust l'accepte à l'intérieur d'un crate. Elle rendrait impossible l'extraction de `pbc` en
  crate séparé.
* **B — un module `fault` à la racine**, sans dépendance, importé par tous. Un fichier de 100 lignes, une
  `enum`, un `as_str`. La liste close de B.5 est visible à un seul endroit.
* **C — `pbc` a son propre `LoadError`** (`BadMagic`, `BadVersion`, `Truncated`…) converti en `Fault::BadHeader`
  par l'appelant. Plus précis pour un message d'erreur, mais A.2 dit explicitement que toute violation est
  `BadHeader` : le détail n'est jamais observable, et il faudrait maintenir une conversion.

### Forme de `Program`

* **A — possède ses données** (`Vec<i64>`, `Vec<u8>`), champs privés, construit uniquement par `load` ou `new`
  qui vérifient les bornes. Un `Program` invalide est inreprésentable. Une copie des octets reçus, bornée à
  98 307 octets.
* **B — emprunte** (`Program<'a>` sur `&'a [u8]`), zéro copie. Mais une session réseau doit garder le programme
  après avoir rendu son tampon de trame : il faudrait de toute façon le copier quelque part, et la durée de vie
  contaminerait `Machine`, les sessions et les tests.

### Garantie « aucune allocation non bornée »

* **A — vérifier chaque borne avant d'allouer** : `const_count <= 4095` avant de lire les constantes,
  `code_len` dans `1..=65535` **et** `reste == code_len` avant de copier le code. L'allocation maximale est
  alors celle d'un fichier légal.
* **B — refuser d'emblée tout tampon de plus de 98 307 octets**, puis allouer. Plus court, mais un en-tête qui
  annonce 4095 constantes sur un fichier de 20 octets passerait la première garde et il faudrait encore
  vérifier les longueurs une à une.

## 3. Ce que l'assistant IA a proposé

Claude Code a proposé et mis en place **B pour `Fault`, A pour `Program`, A pour les allocations**, avec ces
détails :

* `Fault` dérive `Serialize`/`Deserialize` de `serde`, parce que le nom de variante est exactement la chaîne
  normative ; un test vérifie que `Display`, `as_str` et le JSON rendent la même chaîne pour les seize, et qu'une
  chaîne inconnue est refusée au décodage JSON ;
* `Fault::ALL`, un tableau des seize, pour que les tests prouvent que la liste est close ;
* un chargeur sans aucune indexation `[i]` : il avance par `split_first_chunk::<N>()` et `split_at_checked`,
  qui rendent `None` au lieu de paniquer, et convertit les constantes par `as_chunks::<8>()`. Ces trois méthodes
  demandent Rust 1.88, d'où `rust-version = "1.88"` dans `Cargo.toml` ;
* `Program::new` et `Program::encode`, pour que l'assembleur et les tests fabriquent du bytecode sans écrire les
  octets à la main, et pour tester la propriété `load(encode(p)) == p` avec `proptest` ;
* un décodeur `decode(code, pc) -> Result<Instr, Fault>` qui applique les étapes 1 à 3 de B.2 dans l'ordre
  (`PcOutOfRange`, puis `BadOpcode`, puis `Truncated`), et `Instr::size()` pour l'avance du `pc` et la base des
  sauts (B.4). Le décodeur ne lit que les octets de l'instruction visée, jamais ce qui précède (A.3) ;
* `JUMPDEST` exposé comme constante d'octet, parce que B.4 demande de lire **l'octet** à la cible, pas de
  décoder une instruction ;
* la table de gaz **n'est pas** dans `pbc` : elle relève de l'annexe D et vivra dans `vm`, à un seul endroit.

L'assistant a signalé un doublon assumé : `decode` et `Instr::opcode` écrivent chacun la table des 43 opcodes.
Un test fait l'aller-retour sur les 43 pour garantir qu'elles restent synchrones. L'alternative, une table
unique de `(opcode, constructeur)`, aurait été plus difficile à lire en soutenance.

**Ce que nous avons vérifié** : _à compléter — par exemple : relire le test `unknown_opcodes_are_bad_opcode`,
qui balaie les 256 octets ; lancer `cargo test` et constater que les 65 programmes du corpus sont acceptés ou
refusés comme `case.toml` le dit ; vérifier à la main que le cas `003-header-trailing-byte` échoue bien sur
`reste.len() != code_len`._

## 4. Ce que nous avons décidé, et pourquoi

_À compléter par l'équipe au moment d'adopter._

## 5. Ce qui nous ferait changer d'avis

* Un besoin de messages d'erreur détaillés au chargement (outil `disasm`, assembleur) : on ajouterait un type
  de diagnostic **à côté** de `Fault`, jamais une dix-septième variante.
* La version 2.0.0 de la spec (H.2) qui ajouterait une faute : c'est `fault.rs` seul qui change, et le test
  `the_list_is_closed_at_sixteen` qui passe à dix-sept — c'est précisément le signal attendu.
* Une mesure montrant que la copie des octets dans `Program` coûte quelque chose de visible : improbable sous
  98 Ko, mais ce serait le moment de revoir l'option « emprunte ».
* Un fuzzing (`cargo-fuzz`) qui trouverait un `panic` dans `load` ou `decode` : la décision tient, mais le
  code serait révisé et le cas ajouté aux tests.

## Révisions

_(aucune)_
