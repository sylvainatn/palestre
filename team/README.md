# Équipe — répartition des tâches

Répartition posée le 2026-10-18. Chaque membre **possède** une zone : il en écrit ou relit le code, il en porte les
ADR dans `journal/`, et il sait la défendre en soutenance. Chacun doit aussi pouvoir expliquer les deux autres zones à
l'oral : le jury tire une décision au hasard (README §5).

| Membre  | Zone                           | Fichier                    |
|---------|--------------------------------|----------------------------|
| Sylvain | `pbc` et le harnais de corpus  | [sylvain.md](sylvain.md)   |
| Shany   | `vm`                           | [shany.md](shany.md)       |
| Vincent | `arena` puis `net`             | [vincent.md](vincent.md)   |

Ce dossier sert à **s'organiser**. Il ne remplace pas le journal : toute décision d'architecture va dans
`journal/D-NNN-*.md`, avant ou avec le code qu'elle motive.

## Jalons (README §4)

| Jalon | Attendu                                                              | Zone principale  |
|-------|----------------------------------------------------------------------|------------------|
| J1    | CI, chargeur PBC, décodeur                                           | Sylvain (fait)   |
| J2    | Machine complète : pile, arithmétique, gaz, sauts, appels, 16 fautes | Shany (fait, ADR à adopter) |
| J3    | Serveur TCP de l'annexe K, rappels `SENSE`/`ACT`/`RAND`              | Vincent          |
| J4    | Journée d'interopérabilité en salle                                  | tous             |
| J5    | Tests consolidés (propriétés, fuzzing), documentation                | tous             |

Cocher une case quand la tâche est **mergée sur `master`**, pas seulement écrite sur une branche.
