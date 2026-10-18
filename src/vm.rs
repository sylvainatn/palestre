//! Machine virtuelle — annexes B et D.
//!
//! Responsabilités de ce module :
//!
//! * les ressources de B.1 en tableaux de taille fixe : pile de 64 `i64`, pile d'appels de
//!   16 adresses, mémoire de 256 cellules, budget de gaz `u16` ;
//! * le cycle d'exécution de B.2, dans l'ordre normatif : `PcOutOfRange`, décodage, `Truncated`,
//!   **prélèvement du gaz**, exécution, avance du `pc` ;
//! * la sémantique des 43 instructions de B.3, les sauts de B.4, les décalages de B.6, `GAS` de B.7 ;
//! * la liste **close** des seize fautes de B.5, dont l'affichage rend la chaîne exacte de la spec ;
//! * la table de gaz de l'annexe D, écrite à **un seul endroit**.
//!
//! La machine ne connaît aucun monde : `SENSE`, `ACT` et `RAND` sont délégués à un
//! environnement fourni par l'appelant (arène locale, serveur réseau ou harnais de test).
//! Aucune allocation n'a lieu pendant un tour (README §3).

/// Nombre de cellules de la mémoire d'un agent, indices `0..=255` (B.1).
pub const MEM_SIZE: usize = 256;
