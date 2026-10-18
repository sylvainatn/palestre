//! Arène locale — annexe E.
//!
//! Responsabilités de ce module :
//!
//! * le monde : grille `0..=x_max` × `0..=y_max`, murs, ressources, agents ;
//! * le déroulement d'un tour (E.2) : instantané, exécution, **restauration sur faute**,
//!   décrément d'énergie, mort ;
//! * les capteurs (E.3) et les actions (E.4) répondus à la machine ;
//! * la fin de partie et le départage (E.5).
//!
//! C'est ici, et non dans la machine, que l'atomicité du tour (invariant 3) est appliquée :
//! la machine rend sa mémoire telle quelle, l'arène décide de la garder ou de la jeter.
