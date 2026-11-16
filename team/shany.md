# Shany — `vm`

## Fait (sur `master` depuis `d7bacf3`)

- [x] D-004 : contrat du trait `Env` (`SENSE`, `ACT`, `RAND`)
- [x] D-005 : état de la machine et cycle B.2
- [x] Table de gaz centralisée, tests `OutOfGas`
- [x] D-006 : pile, arithmétique enveloppante, logique, comparaisons
- [x] D-007 : mémoire, sauts (B.4, `JUMPDEST`), appels
- [x] D-008 : branchement de `SENSE`, `ACT`, `RAND` sur `Env`, trace

## À faire

- [ ] Faire passer D-004 à D-008 de `proposée` à `adoptée` avec l'équipe (section 4 de chaque ADR)
- [ ] Vérifier que les 16 fautes de B.5 sont couvertes par au moins un test chacune
- [ ] Faire passer tous les cas `corpus/` dès que le harnais de Sylvain existe

## Relectures

- Relire le harnais de corpus (Sylvain) et l'utilisation de `Env` par l'arène et le serveur (Vincent)
