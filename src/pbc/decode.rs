//! Décodeur d'instructions — annexe B.3, et les trois premières étapes de B.2.

use crate::fault::Fault;

/// L'opcode de `JUMPDEST`, le seul octet qu'un saut a le droit de viser (A.3, B.4).
///
/// La machine le lit **directement dans le code**, à l'adresse visée, sans décoder quoi que ce
/// soit d'autre : c'est pour cela qu'il est exposé comme un octet et non comme une [`Instr`].
pub const JUMPDEST: u8 = 0x43;

/// Une instruction décodée, avec son opérande immédiat s'il en a un (B.3).
///
/// Les opérandes gardent le type exact de la table : `u16` pour `CONST`, `i16` étendu en signe
/// pour `PUSHI16` et les déplacements de saut, `u8` pour `LOAD`/`STORE`/`SENSE`/`ACT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instr {
    /// `0x00` — fin normale du tour.
    Halt,
    /// `0x01 k` — empile `constants[k]`.
    Const(u16),
    /// `0x02 n` — empile `n` étendu en signe.
    PushI16(i16),
    /// `0x03` — `a --`.
    Drop,
    /// `0x04` — `a -- a a`.
    Dup,
    /// `0x05` — `a b -- b a`.
    Swap,
    /// `0x06` — `a b -- a b a`.
    Over,
    /// `0x07` — `a b c -- b c a`.
    Rot,
    /// `0x08` — `k -- constants[k]`, index dépilé.
    ConstX,
    /// `0x10` — addition enveloppante.
    Add,
    /// `0x11` — soustraction enveloppante.
    Sub,
    /// `0x12` — multiplication enveloppante.
    Mul,
    /// `0x13` — division tronquée vers zéro.
    Div,
    /// `0x14` — reste, du signe du dividende.
    Mod,
    /// `0x15` — négation enveloppante.
    Neg,
    /// `0x20` — égalité, rend `0` ou `1`.
    Eq,
    /// `0x21` — `a < b` signé.
    Lt,
    /// `0x22` — `a > b` signé.
    Gt,
    /// `0x23` — non **logique** : `1` si `a == 0`.
    Not,
    /// `0x24` — et bit à bit.
    And,
    /// `0x25` — ou bit à bit.
    Or,
    /// `0x26` — ou exclusif bit à bit.
    Xor,
    /// `0x27` — décalage logique à gauche.
    Shl,
    /// `0x28` — décalage logique à droite.
    Shr,
    /// `0x29` — complément à un, **bit à bit**.
    BNot,
    /// `0x2D` — décalage arithmétique à droite.
    Sar,
    /// `0x30` — `addr -- mem[addr]`.
    LoadX,
    /// `0x31` — `v addr --`, `mem[addr] = v`.
    StoreX,
    /// `0x32 k` — empile `mem[k]`.
    Load(u8),
    /// `0x33 k` — `v --`, `mem[k] = v`.
    Store(u8),
    /// `0x40 d` — saut relatif.
    Jmp(i16),
    /// `0x41 d` — saute si le sommet dépilé vaut `0`.
    Jz(i16),
    /// `0x42 d` — saute si le sommet dépilé ne vaut pas `0`.
    Jnz(i16),
    /// `0x43` — cible de saut valide ; aucun autre effet.
    JumpDest,
    /// `0x44` — `t --`, saut à la cible absolue dépilée.
    JmpX,
    /// `0x50 d` — empile l'adresse de retour, saut relatif.
    Call(i16),
    /// `0x51` — dépile l'adresse de retour.
    Return,
    /// `0x52` — `t --`, empile l'adresse de retour, saut absolu.
    CallX,
    /// `0x60 k` — capteur `k` (E.3).
    Sense(u8),
    /// `0x61 k` — action `k` (E.4), `arg -- ok`.
    Act(u8),
    /// `0x70` — valeur aléatoire fournie par l'environnement.
    Rand,
    /// `0x71` — gaz restant après son propre coût (B.7).
    Gas,
    /// `0xF0` — `v --`, ajoute `v` à la trace.
    Trace,
}

impl Instr {
    /// Taille de l'instruction en octets, opcode compris : `1`, `2` ou `3` (B.3).
    ///
    /// C'est de cette taille que `pc` avance (B.2, étape 6) et que partent les sauts
    /// relatifs : `cible = pc + taille + d` (B.4).
    pub const fn size(self) -> usize {
        match self {
            Instr::Const(_)
            | Instr::PushI16(_)
            | Instr::Jmp(_)
            | Instr::Jz(_)
            | Instr::Jnz(_)
            | Instr::Call(_) => 3,
            Instr::Load(_) | Instr::Store(_) | Instr::Sense(_) | Instr::Act(_) => 2,
            _ => 1,
        }
    }

    /// L'octet d'opcode de B.3. Inverse de [`decode`] sur le premier octet ; sert à l'assembleur.
    pub const fn opcode(self) -> u8 {
        match self {
            Instr::Halt => 0x00,
            Instr::Const(_) => 0x01,
            Instr::PushI16(_) => 0x02,
            Instr::Drop => 0x03,
            Instr::Dup => 0x04,
            Instr::Swap => 0x05,
            Instr::Over => 0x06,
            Instr::Rot => 0x07,
            Instr::ConstX => 0x08,
            Instr::Add => 0x10,
            Instr::Sub => 0x11,
            Instr::Mul => 0x12,
            Instr::Div => 0x13,
            Instr::Mod => 0x14,
            Instr::Neg => 0x15,
            Instr::Eq => 0x20,
            Instr::Lt => 0x21,
            Instr::Gt => 0x22,
            Instr::Not => 0x23,
            Instr::And => 0x24,
            Instr::Or => 0x25,
            Instr::Xor => 0x26,
            Instr::Shl => 0x27,
            Instr::Shr => 0x28,
            Instr::BNot => 0x29,
            Instr::Sar => 0x2D,
            Instr::LoadX => 0x30,
            Instr::StoreX => 0x31,
            Instr::Load(_) => 0x32,
            Instr::Store(_) => 0x33,
            Instr::Jmp(_) => 0x40,
            Instr::Jz(_) => 0x41,
            Instr::Jnz(_) => 0x42,
            Instr::JumpDest => JUMPDEST,
            Instr::JmpX => 0x44,
            Instr::Call(_) => 0x50,
            Instr::Return => 0x51,
            Instr::CallX => 0x52,
            Instr::Sense(_) => 0x60,
            Instr::Act(_) => 0x61,
            Instr::Rand => 0x70,
            Instr::Gas => 0x71,
            Instr::Trace => 0xF0,
        }
    }

    /// Les octets de l'instruction, opcode puis opérande gros-boutiste. Inverse de [`decode`].
    pub fn encode(self, out: &mut Vec<u8>) {
        out.push(self.opcode());
        match self {
            Instr::Const(k) => out.extend_from_slice(&k.to_be_bytes()),
            Instr::PushI16(n) | Instr::Jmp(n) | Instr::Jz(n) | Instr::Jnz(n) | Instr::Call(n) => {
                out.extend_from_slice(&n.to_be_bytes());
            }
            Instr::Load(k) | Instr::Store(k) | Instr::Sense(k) | Instr::Act(k) => out.push(k),
            _ => {}
        }
    }
}

/// Lit les `N` octets d'opérande qui commencent à `at`, ou `Truncated` s'ils dépassent le code
/// (B.2, étape 3).
fn operand<const N: usize>(code: &[u8], at: usize) -> Result<[u8; N], Fault> {
    code.get(at..)
        .and_then(|rest| rest.first_chunk::<N>())
        .copied()
        .ok_or(Fault::Truncated)
}

/// Décode l'instruction qui commence à `pc`, dans l'ordre des étapes 1 à 3 de B.2 :
///
/// 1. `pc >= code.len()` ⇒ `PcOutOfRange` ;
/// 2. opcode hors de B.3 ⇒ `BadOpcode` ;
/// 3. opérande coupé par la fin du code ⇒ `Truncated`.
///
/// Le décodage ne regarde que les octets de l'instruction elle-même (A.3).
pub fn decode(code: &[u8], pc: usize) -> Result<Instr, Fault> {
    let opcode = *code.get(pc).ok_or(Fault::PcOutOfRange)?;
    // `pc < code.len() <= 65535` : `pc + 1` ne déborde pas.
    let at = pc + 1;
    let instr = match opcode {
        0x00 => Instr::Halt,
        0x01 => Instr::Const(u16::from_be_bytes(operand(code, at)?)),
        0x02 => Instr::PushI16(i16::from_be_bytes(operand(code, at)?)),
        0x03 => Instr::Drop,
        0x04 => Instr::Dup,
        0x05 => Instr::Swap,
        0x06 => Instr::Over,
        0x07 => Instr::Rot,
        0x08 => Instr::ConstX,
        0x10 => Instr::Add,
        0x11 => Instr::Sub,
        0x12 => Instr::Mul,
        0x13 => Instr::Div,
        0x14 => Instr::Mod,
        0x15 => Instr::Neg,
        0x20 => Instr::Eq,
        0x21 => Instr::Lt,
        0x22 => Instr::Gt,
        0x23 => Instr::Not,
        0x24 => Instr::And,
        0x25 => Instr::Or,
        0x26 => Instr::Xor,
        0x27 => Instr::Shl,
        0x28 => Instr::Shr,
        0x29 => Instr::BNot,
        0x2D => Instr::Sar,
        0x30 => Instr::LoadX,
        0x31 => Instr::StoreX,
        0x32 => Instr::Load(operand::<1>(code, at)?[0]),
        0x33 => Instr::Store(operand::<1>(code, at)?[0]),
        0x40 => Instr::Jmp(i16::from_be_bytes(operand(code, at)?)),
        0x41 => Instr::Jz(i16::from_be_bytes(operand(code, at)?)),
        0x42 => Instr::Jnz(i16::from_be_bytes(operand(code, at)?)),
        JUMPDEST => Instr::JumpDest,
        0x44 => Instr::JmpX,
        0x50 => Instr::Call(i16::from_be_bytes(operand(code, at)?)),
        0x51 => Instr::Return,
        0x52 => Instr::CallX,
        0x60 => Instr::Sense(operand::<1>(code, at)?[0]),
        0x61 => Instr::Act(operand::<1>(code, at)?[0]),
        0x70 => Instr::Rand,
        0x71 => Instr::Gas,
        0xF0 => Instr::Trace,
        _ => return Err(Fault::BadOpcode),
    };
    Ok(instr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Les 43 instructions de B.3, une fois chacune, avec un opérande non trivial.
    const ALL: [Instr; 43] = [
        Instr::Halt,
        Instr::Const(0x0102),
        Instr::PushI16(-2),
        Instr::Drop,
        Instr::Dup,
        Instr::Swap,
        Instr::Over,
        Instr::Rot,
        Instr::ConstX,
        Instr::Add,
        Instr::Sub,
        Instr::Mul,
        Instr::Div,
        Instr::Mod,
        Instr::Neg,
        Instr::Eq,
        Instr::Lt,
        Instr::Gt,
        Instr::Not,
        Instr::And,
        Instr::Or,
        Instr::Xor,
        Instr::Shl,
        Instr::Shr,
        Instr::BNot,
        Instr::Sar,
        Instr::LoadX,
        Instr::StoreX,
        Instr::Load(255),
        Instr::Store(7),
        Instr::Jmp(-13),
        Instr::Jz(300),
        Instr::Jnz(i16::MIN),
        Instr::JumpDest,
        Instr::JmpX,
        Instr::Call(i16::MAX),
        Instr::Return,
        Instr::CallX,
        Instr::Sense(13),
        Instr::Act(3),
        Instr::Rand,
        Instr::Gas,
        Instr::Trace,
    ];

    #[test]
    fn every_instruction_decodes_back_to_itself() {
        for instr in ALL {
            let mut bytes = Vec::new();
            instr.encode(&mut bytes);
            assert_eq!(bytes.len(), instr.size(), "{instr:?}");
            assert_eq!(bytes[0], instr.opcode(), "{instr:?}");
            assert_eq!(decode(&bytes, 0), Ok(instr), "{instr:?}");
        }
    }

    #[test]
    fn operands_are_big_endian() {
        assert_eq!(decode(&[0x01, 0x01, 0x02], 0), Ok(Instr::Const(258)));
        assert_eq!(decode(&[0x02, 0xFF, 0xFF], 0), Ok(Instr::PushI16(-1)));
        assert_eq!(decode(&[0x40, 0xFF, 0xF3], 0), Ok(Instr::Jmp(-13)));
    }

    #[test]
    fn pc_past_the_end_is_pc_out_of_range() {
        assert_eq!(decode(&[0x00], 1), Err(Fault::PcOutOfRange));
        assert_eq!(decode(&[], 0), Err(Fault::PcOutOfRange));
        assert_eq!(decode(&[0x00], usize::MAX), Err(Fault::PcOutOfRange));
    }

    #[test]
    fn unknown_opcodes_are_bad_opcode() {
        for opcode in 0..=255u8 {
            let known = ALL.iter().any(|i| i.opcode() == opcode);
            let result = decode(&[opcode, 0, 0], 0);
            assert_eq!(result.is_ok(), known, "opcode {opcode:#04x}");
            if !known {
                assert_eq!(result, Err(Fault::BadOpcode));
            }
        }
    }

    #[test]
    fn a_cut_operand_is_truncated() {
        assert_eq!(decode(&[0x02], 0), Err(Fault::Truncated));
        assert_eq!(decode(&[0x02, 0x00], 0), Err(Fault::Truncated));
        assert_eq!(decode(&[0x60], 0), Err(Fault::Truncated));
        assert_eq!(decode(&[0x00, 0x42, 0x00], 1), Err(Fault::Truncated));
    }

    #[test]
    fn bad_opcode_wins_over_truncated() {
        // Étape 2 avant étape 3 : un octet inconnu en fin de code est BadOpcode, pas Truncated.
        assert_eq!(decode(&[0xFF], 0), Err(Fault::BadOpcode));
    }

    #[test]
    fn decoding_is_local_a_jumpdest_inside_an_operand_is_a_jumpdest() {
        // PUSHI16 67 : l'opérande est 0x00 0x43 ; décoder à l'octet 2 donne JUMPDEST (A.3).
        let code = [0x02, 0x00, 0x43, 0x00];
        assert_eq!(decode(&code, 0), Ok(Instr::PushI16(67)));
        assert_eq!(decode(&code, 2), Ok(Instr::JumpDest));
        assert_eq!(code[2], JUMPDEST);
    }

    proptest! {
        /// Invariant 5 : décoder à n'importe quel décalage de n'importe quel code ne panique pas.
        #[test]
        fn decode_never_panics(code in prop::collection::vec(any::<u8>(), 0..64), pc in 0usize..80) {
            let _ = decode(&code, pc);
        }
    }
}
