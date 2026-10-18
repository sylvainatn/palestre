# CLAUDE.md — Palestre

@AGENT.md

Les règles communes à tout assistant sont dans `AGENT.md` ci-dessus. Ce fichier ajoute ce qui est propre à
Claude Code dans ce dépôt.

## Langue et ton

Répondre en français. Les auteurs sont des étudiants qui devront expliquer ce code en soutenance : quand tu écris
du Rust, dis en une ou deux phrases *pourquoi* c'est écrit ainsi et quelle clause de `SPEC-PBC-v1.md` il applique
(`B.2`, `B.4`, `K.4`…). Une référence de clause vaut mieux qu'un long commentaire.

## Avant d'écrire du code

1. Relire la clause de la spec concernée, pas le README. En cas de contradiction, la spec gagne.
2. Chercher dans `corpus/INDEX.md` les cas qui éprouvent cette clause, et viser à les faire passer.
3. Si la tâche est une décision d'architecture (découpage, type, dépendance, protocole, stratégie de test),
   présenter **deux options minimum** et proposer un brouillon `journal/D-NNN-*.md` avant le code. L'équipe
   choisit ; tu ne commites pas la décision à leur place.

## Ce que tu ne fais jamais sans demander

* Ajouter une dépendance hors de la liste du README §3, même « juste pour les tests ».
* Introduire `unsafe`, `async`, `unwrap`/`expect`/`panic!` hors des tests, ou une allocation dans la boucle de tour.
* Modifier `SPEC-PBC-v1.md`, `README.md` ou un fichier de `corpus/`.
* Commiter, pousser, ou réécrire l'historique. Les commits sont nominatifs et appartiennent aux étudiants.
* Rédiger ou dater un ADR après coup pour « rattraper » une décision déjà codée.

## Vérification avant de rendre la main

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

Si une commande échoue, le dire avec la sortie, ne pas contourner. Si un cas du corpus échoue, rapporter le
premier champ divergent dans l'ordre rappels → `gas_used` → `fault` → `act` → `trace` → `mem`.

## Conventions de nommage

* Crate bibliothèque `palestre` avec les modules `fault`, `pbc`, `vm`, `arena`, `net`, `corpus`, `asm` ; binaire `palestre` dans
  `src/main.rs` ou `src/bin/`.
* `Fault` est un `enum` fermé à 16 variantes, dans l'ordre de B.5, dont le `Display` rend la chaîne exacte de la
  spec. Pas de 17ᵉ variante, jamais.
* Le numéro de version de la spec et la table de gaz vivent chacun à un seul endroit (annexe H.2).
