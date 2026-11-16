# Vincent — `arena` puis `net`

## `arena` (annexe E)

- [ ] Grille, agents, ressources, murs
- [ ] Déroulement d'un tour E.2 : instantané de la mémoire, restauration sur faute
- [ ] Capteurs `SENSE` (E.3) et actions `ACT` (E.4), en implémentant le trait `Env` de Shany
- [ ] Arbitrage et victoire (E.5)
- [ ] Match de démonstration `idler` contre un agent mobile (MVP README §2, point 3)

## `net` (annexe K) — obligatoire

- [ ] Cadrage des trames K.2 (big-endian, borne de taille)
- [ ] Trames JSON K.3 avec `serde_json`
- [ ] Machine d'états K.4 par connexion (`OPEN`, `SUBMIT`, `EXEC`, rappels)
- [ ] `palestre exec --listen <adresse>` (serveur séquentiel suffisant)
- [ ] Préparer la journée d'interopérabilité J4

## Décisions à ouvrir dans `journal/`

- Représentation de la grille et de l'instantané E.2
- Découpage du serveur et gestion des erreurs réseau

## Relectures

- Relire D-004 (contrat `Env`) de Shany avant de coder l'arène
