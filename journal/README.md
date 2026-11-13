# Journal de décisions d'architecture (ADR)

Un fichier par décision, nommé `D-NNN-titre-court.md`, numéroté dans l'ordre d'ouverture. Le gabarit est
`TEMPLATE.md`. Les règles complètes sont dans `README.md` §4 à la racine.

Règles que ce journal s'impose :

* **Antériorité** : une entrée est commitée **avant ou avec** le code qu'elle motive, jamais après.
* **Statuts** : `proposée` → `adoptée`, puis éventuellement `révisée` ou `abandonnée`. On ne réécrit pas une
  décision qui a changé : on la passe en `révisée` et on ajoute ce qui l'a fait bouger, en gardant l'ancien texte.
* **Honnêteté de la section 3** : ce que l'assistant IA a proposé est retranscrit tel quel, y compris quand c'était
  faux ou quand on l'a suivi sans discuter. Dire ce qu'on a vérifié.
* **Un bonus, un ADR** : sans entrée dédiée, un bonus ne compte pas (README §6).

## Index

| N°    | Titre                                                  | Statut     | Date       |
|-------|--------------------------------------------------------|------------|------------|
| D-001 | [Structure du projet Cargo](D-001-structure-du-projet.md) | `proposée` | 2026-10-18 |
| D-002 | [Où vit `Fault`, et la forme du chargeur](D-002-fault-partage-et-chargeur.md) | `proposée` | 2026-10-18 |
| D-003 | [Lire `case.toml` sans la crate `toml`](D-003-lire-case-toml-sans-la-crate-toml.md) | `proposée` | 2026-10-18 |
| D-004 | [Contrat entre la VM et son environnement](D-004-contrat-env.md) | `proposée` | 2026-11-13 |

La [revue du 2026-11-13](verification-2026-11-13.md) complète D-001 à D-003 sans modifier leur date d'ouverture ni consigner une adoption humaine. D-004 prépare le contrat `Env` avant l'implémentation de la VM.
