//! Les seize fautes de la spécification — annexe B.5.
//!
//! La liste est **close** : seize variantes, pas une de plus (B.5 le répète à chaque ajout
//! d'instruction). Les chaînes rendues par [`Fault::as_str`] sont normatives : elles voyagent
//! telles quelles dans le champ `fault` des `RESULT` (K.3) et des `expected.json` du corpus.
//!
//! Le module est partagé par le chargeur (`BadHeader`), le décodeur (`BadOpcode`, `Truncated`,
//! `PcOutOfRange`) et la machine (tout le reste) : voir `journal/D-002`.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Une faute d'exécution ou de chargement, dans l'ordre de la liste de B.5.
///
/// `Serialize`/`Deserialize` produisent le nom de la variante, qui est exactement la chaîne
/// normative : `"OutOfGas"` sur le fil, `Fault::OutOfGas` en mémoire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Fault {
    /// Le fichier `.pbc` viole l'annexe A.1. Erreur de **chargement**, jamais une faute de tour.
    BadHeader,
    /// `CONST k` ou `CONSTX` hors de la table des constantes (B.3).
    BadConst,
    /// Octet qui n'est l'opcode d'aucune instruction de B.3 (B.2, étape 2).
    BadOpcode,
    /// Opérande coupé par la fin du code (B.2, étape 3).
    Truncated,
    /// `pc >= code_len` au début d'un pas (B.2, étape 1).
    PcOutOfRange,
    /// Cible de saut hors du code, ou qui ne porte pas le marqueur `JUMPDEST` (B.4).
    BadJump,
    /// Dépile sur une pile de données vide (B.1).
    StackUnderflow,
    /// Empile sur une pile de données pleine, 64 emplacements (B.1).
    StackOverflow,
    /// Dix-septième adresse de retour (B.1).
    CallStackOverflow,
    /// `RETURN` sans adresse de retour (B.3).
    CallStackUnderflow,
    /// Gaz restant insuffisant pour l'instruction, qui n'est alors pas exécutée (B.2, étape 4).
    OutOfGas,
    /// `DIV` ou `MOD` par zéro (B.3).
    DivByZero,
    /// Accès mémoire hors de `0..=255` par `LOADX`/`STOREX` (B.1).
    OutOfBounds,
    /// Capteur inconnu, de la machine ou de l'environnement (E.3).
    BadSensor,
    /// Action inconnue, hors de `0..=3` (E.4).
    BadAction,
    /// Second `ACT` du même tour (E.4).
    AlreadyActed,
}

impl Fault {
    /// Les seize fautes, dans l'ordre de B.5. Sert aux tests d'exhaustivité.
    pub const ALL: [Fault; 16] = [
        Fault::BadHeader,
        Fault::BadConst,
        Fault::BadOpcode,
        Fault::Truncated,
        Fault::PcOutOfRange,
        Fault::BadJump,
        Fault::StackUnderflow,
        Fault::StackOverflow,
        Fault::CallStackOverflow,
        Fault::CallStackUnderflow,
        Fault::OutOfGas,
        Fault::DivByZero,
        Fault::OutOfBounds,
        Fault::BadSensor,
        Fault::BadAction,
        Fault::AlreadyActed,
    ];

    /// La chaîne exacte de B.5, celle qui apparaît sur le fil et dans le corpus.
    pub const fn as_str(self) -> &'static str {
        match self {
            Fault::BadHeader => "BadHeader",
            Fault::BadConst => "BadConst",
            Fault::BadOpcode => "BadOpcode",
            Fault::Truncated => "Truncated",
            Fault::PcOutOfRange => "PcOutOfRange",
            Fault::BadJump => "BadJump",
            Fault::StackUnderflow => "StackUnderflow",
            Fault::StackOverflow => "StackOverflow",
            Fault::CallStackOverflow => "CallStackOverflow",
            Fault::CallStackUnderflow => "CallStackUnderflow",
            Fault::OutOfGas => "OutOfGas",
            Fault::DivByZero => "DivByZero",
            Fault::OutOfBounds => "OutOfBounds",
            Fault::BadSensor => "BadSensor",
            Fault::BadAction => "BadAction",
            Fault::AlreadyActed => "AlreadyActed",
        }
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for Fault {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_closed_at_sixteen() {
        assert_eq!(Fault::ALL.len(), 16);
    }

    #[test]
    fn display_and_json_agree_on_the_normative_string() {
        for fault in Fault::ALL {
            let json = serde_json::to_string(&fault).unwrap();
            assert_eq!(json, format!("\"{}\"", fault.as_str()));
            assert_eq!(fault.to_string(), fault.as_str());
            let back: Fault = serde_json::from_str(&json).unwrap();
            assert_eq!(back, fault);
        }
    }

    #[test]
    fn an_unknown_string_is_not_a_fault() {
        assert!(serde_json::from_str::<Fault>("\"BadMagic\"").is_err());
    }
}
