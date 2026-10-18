//! Palestre — machine à agents déterministe.
//!
//! Cette bibliothèque implémente la spécification normative `SPEC-PBC-v1.md` :
//!
//! * [`fault`] — les seize fautes de B.5, partagées par tous les modules ;
//! * [`pbc`]   — le format binaire `.pbc` (annexe A) : chargeur et décodeur ;
//! * [`vm`]    — la machine à pile, le gaz et les seize fautes (annexes B et D) ;
//! * [`arena`] — le monde, les tours et les règles de victoire (annexe E) ;
//! * [`net`]   — le protocole d'exécution distante (annexe K) ;
//! * [`corpus`] — la lecture des cas de test de `corpus/` (hors machine, pour les tests et le rejeu).
//!
//! Le binaire `palestre` (`src/main.rs`) ne fait qu'assembler ces modules.

// Invariant 5 (README §1, annexe A.2) : la bibliothèque ne panique jamais.
// Ces lints sont des erreurs hors des tests ; les tests ont le droit à `unwrap`.
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented,
        clippy::indexing_slicing
    )
)]

pub mod arena;
pub mod corpus;
pub mod fault;
pub mod net;
pub mod pbc;
pub mod vm;

/// Version de la spécification implémentée (`SPEC-PBC-v1.md`, en-tête et annexe H).
///
/// C'est le **seul** endroit du code où ce numéro est écrit : il est repris tel quel
/// dans la trame `READY` (K.3) et comparé au `spec_version` des `expected.json` du corpus.
/// L'annexe H.2 annonce une version `2.0.0` révélée en soutenance ; la changer ici doit suffire.
pub const SPEC_VERSION: &str = "1.9.0";
