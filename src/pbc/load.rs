//! Chargeur du fichier `.pbc` — annexe A.1 et A.2.

use crate::fault::Fault;

/// Le nombre magique en tête de tout fichier `.pbc` : `"PBC1"` (A.1).
pub const MAGIC: [u8; 4] = *b"PBC1";
/// Seule version de format acceptée (A.1).
pub const FORMAT_VERSION: u8 = 0x01;
/// Taille de l'en-tête fixe : magic, version, flags, `const_count` (A.1).
pub const HEADER_LEN: usize = 8;
/// Nombre maximal de constantes (A.1).
pub const MAX_CONST_COUNT: usize = 4095;
/// Taille minimale du code, en octets (A.1).
pub const MIN_CODE_LEN: usize = 1;
/// Taille maximale du code, en octets (A.1).
pub const MAX_CODE_LEN: usize = 65535;
/// Taille maximale d'un fichier `.pbc` : `8 + 8 × 4095 + 4 + 65535 = 98307` (A.1, K.2).
pub const MAX_FILE_LEN: usize = HEADER_LEN + 8 * MAX_CONST_COUNT + 4 + MAX_CODE_LEN;

/// Un programme chargé et validé : il respecte toutes les bornes de A.1 par construction.
///
/// Les champs sont privés : on ne peut obtenir un `Program` que par [`Program::load`] ou
/// [`Program::new`], qui vérifient les bornes. Un `Program` invalide est inreprésentable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    constants: Vec<i64>,
    code: Vec<u8>,
}

/// Lit `N` octets en tête de `input`, ou `BadHeader` s'il en manque (A.2 : jamais hors limites).
fn take<const N: usize>(input: &[u8]) -> Result<([u8; N], &[u8]), Fault> {
    input
        .split_first_chunk::<N>()
        .map(|(head, rest)| (*head, rest))
        .ok_or(Fault::BadHeader)
}

impl Program {
    /// Charge et valide un fichier `.pbc` (A.1). Toute violation rend `Fault::BadHeader` (A.2).
    ///
    /// Les bornes sont vérifiées **avant** chaque allocation : la table des constantes n'est
    /// allouée qu'une fois `const_count <= 4095` vérifié, le code qu'une fois `code_len` et la
    /// taille exacte du fichier vérifiées. Une suite d'octets hostile ne peut donc faire allouer
    /// plus de [`MAX_FILE_LEN`] octets.
    pub fn load(bytes: &[u8]) -> Result<Self, Fault> {
        let (magic, rest) = take::<4>(bytes)?;
        if magic != MAGIC {
            return Err(Fault::BadHeader);
        }
        let ([version], rest) = take::<1>(rest)?;
        if version != FORMAT_VERSION {
            return Err(Fault::BadHeader);
        }
        let ([flags], rest) = take::<1>(rest)?;
        if flags != 0x00 {
            return Err(Fault::BadHeader);
        }

        let (const_count, rest) = take::<2>(rest)?;
        let const_count = usize::from(u16::from_be_bytes(const_count));
        if const_count > MAX_CONST_COUNT {
            return Err(Fault::BadHeader);
        }
        // `const_count <= 4095`, donc `8 * const_count` tient largement dans un `usize`.
        let (raw_constants, rest) = rest
            .split_at_checked(8 * const_count)
            .ok_or(Fault::BadHeader)?;

        let (code_len, rest) = take::<4>(rest)?;
        let code_len = u32::from_be_bytes(code_len) as usize;
        if !(MIN_CODE_LEN..=MAX_CODE_LEN).contains(&code_len) {
            return Err(Fault::BadHeader);
        }
        // « Le fichier DOIT faire exactement la taille impliquée par ses champs » (A.1) :
        // un octet de moins est un code tronqué, un octet de plus est un octet excédentaire.
        if rest.len() != code_len {
            return Err(Fault::BadHeader);
        }

        // Toutes les bornes sont vérifiées : on peut allouer, et au plus MAX_FILE_LEN octets.
        let (chunks, _remainder_always_empty) = raw_constants.as_chunks::<8>();
        let constants = chunks.iter().map(|c| i64::from_be_bytes(*c)).collect();
        Ok(Self {
            constants,
            code: rest.to_vec(),
        })
    }

    /// Construit un programme à partir de ses parties, avec les mêmes bornes que [`Program::load`].
    ///
    /// Sert aux outils qui fabriquent du bytecode (assembleur, tests) sans passer par les octets.
    pub fn new(constants: Vec<i64>, code: Vec<u8>) -> Result<Self, Fault> {
        if constants.len() > MAX_CONST_COUNT || !(MIN_CODE_LEN..=MAX_CODE_LEN).contains(&code.len())
        {
            return Err(Fault::BadHeader);
        }
        Ok(Self { constants, code })
    }

    /// Sérialise le programme au format de A.1.
    ///
    /// Propriété testée : `Program::load(&p.encode()) == Ok(p)` pour tout programme `p`.
    pub fn encode(&self) -> Vec<u8> {
        let mut out =
            Vec::with_capacity(HEADER_LEN + 8 * self.constants.len() + 4 + self.code.len());
        out.extend_from_slice(&MAGIC);
        out.push(FORMAT_VERSION);
        out.push(0x00);
        // `constants.len() <= 4095` par construction : la conversion en u16 ne tronque pas.
        out.extend_from_slice(&(self.constants.len() as u16).to_be_bytes());
        for constant in &self.constants {
            out.extend_from_slice(&constant.to_be_bytes());
        }
        // `code.len() <= 65535` par construction : la conversion en u32 ne tronque pas.
        out.extend_from_slice(&(self.code.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.code);
        out
    }

    /// La table des constantes, indexée par `CONST k` et `CONSTX` (B.3).
    pub fn constants(&self) -> &[i64] {
        &self.constants
    }

    /// Les octets d'instructions.
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// `code_len`, entre `1` et `65535` (A.1). Rendu dans `SESSION` (K.3).
    pub fn code_len(&self) -> usize {
        self.code.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Un fichier valide fabriqué à la main : une constante, deux octets de code.
    fn minimal() -> Vec<u8> {
        let mut bytes = b"PBC1\x01\x00\x00\x01".to_vec();
        bytes.extend_from_slice(&42i64.to_be_bytes());
        bytes.extend_from_slice(&2u32.to_be_bytes());
        bytes.extend_from_slice(&[0xF0, 0x00]);
        bytes
    }

    #[test]
    fn a_minimal_valid_file_loads() {
        let program = Program::load(&minimal()).unwrap();
        assert_eq!(program.constants(), &[42]);
        assert_eq!(program.code(), &[0xF0, 0x00]);
        assert_eq!(program.code_len(), 2);
    }

    #[test]
    fn an_empty_constant_table_is_valid() {
        let bytes = b"PBC1\x01\x00\x00\x00\x00\x00\x00\x01\x00".to_vec();
        let program = Program::load(&bytes).unwrap();
        assert!(program.constants().is_empty());
        assert_eq!(program.code(), &[0x00]);
    }

    #[test]
    fn constants_are_big_endian_two_complement() {
        let mut bytes = b"PBC1\x01\x00\x00\x02".to_vec();
        bytes.extend_from_slice(&[0xFF; 8]); // -1
        bytes.extend_from_slice(&[0x80, 0, 0, 0, 0, 0, 0, 0]); // i64::MIN
        bytes.extend_from_slice(&1u32.to_be_bytes());
        bytes.push(0x00);
        let program = Program::load(&bytes).unwrap();
        assert_eq!(program.constants(), &[-1, i64::MIN]);
    }

    #[test]
    fn empty_input_is_bad_header() {
        assert_eq!(Program::load(&[]), Err(Fault::BadHeader));
    }

    #[test]
    fn wrong_magic_is_bad_header() {
        let mut bytes = minimal();
        bytes[3] = b'2';
        assert_eq!(Program::load(&bytes), Err(Fault::BadHeader));
    }

    #[test]
    fn wrong_version_is_bad_header() {
        let mut bytes = minimal();
        bytes[4] = 0x02;
        assert_eq!(Program::load(&bytes), Err(Fault::BadHeader));
    }

    #[test]
    fn nonzero_flags_is_bad_header() {
        let mut bytes = minimal();
        bytes[5] = 0x01;
        assert_eq!(Program::load(&bytes), Err(Fault::BadHeader));
    }

    #[test]
    fn const_count_above_4095_is_bad_header() {
        let mut bytes = b"PBC1\x01\x00".to_vec();
        bytes.extend_from_slice(&4096u16.to_be_bytes());
        // Peu importe la suite : la borne est violée avant toute lecture des constantes.
        bytes.extend_from_slice(&[0; 64]);
        assert_eq!(Program::load(&bytes), Err(Fault::BadHeader));
    }

    #[test]
    fn truncated_constant_table_is_bad_header() {
        let bytes = minimal();
        // On coupe au milieu de l'unique constante.
        assert_eq!(Program::load(&bytes[..12]), Err(Fault::BadHeader));
    }

    #[test]
    fn missing_code_len_is_bad_header() {
        let bytes = minimal();
        assert_eq!(Program::load(&bytes[..16]), Err(Fault::BadHeader));
        assert_eq!(Program::load(&bytes[..18]), Err(Fault::BadHeader));
    }

    #[test]
    fn zero_code_len_is_bad_header() {
        let bytes = b"PBC1\x01\x00\x00\x00\x00\x00\x00\x00".to_vec();
        assert_eq!(Program::load(&bytes), Err(Fault::BadHeader));
    }

    #[test]
    fn code_len_above_65535_is_bad_header() {
        let mut bytes = b"PBC1\x01\x00\x00\x00".to_vec();
        bytes.extend_from_slice(&65536u32.to_be_bytes());
        bytes.extend_from_slice(&[0; 16]);
        assert_eq!(Program::load(&bytes), Err(Fault::BadHeader));
    }

    #[test]
    fn truncated_code_is_bad_header() {
        let bytes = minimal();
        assert_eq!(
            Program::load(&bytes[..bytes.len() - 1]),
            Err(Fault::BadHeader)
        );
    }

    #[test]
    fn trailing_byte_is_bad_header() {
        let mut bytes = minimal();
        bytes.push(0x00);
        assert_eq!(Program::load(&bytes), Err(Fault::BadHeader));
    }

    #[test]
    fn new_enforces_the_same_bounds_as_load() {
        assert!(Program::new(vec![], vec![0x00]).is_ok());
        assert_eq!(Program::new(vec![], vec![]), Err(Fault::BadHeader));
        assert_eq!(
            Program::new(vec![0; MAX_CONST_COUNT + 1], vec![0x00]),
            Err(Fault::BadHeader)
        );
        assert_eq!(
            Program::new(vec![], vec![0x00; MAX_CODE_LEN + 1]),
            Err(Fault::BadHeader)
        );
    }

    #[test]
    fn the_largest_legal_file_loads_and_is_max_file_len() {
        let program = Program::new(vec![7; MAX_CONST_COUNT], vec![0x43; MAX_CODE_LEN]).unwrap();
        let bytes = program.encode();
        assert_eq!(bytes.len(), MAX_FILE_LEN);
        assert_eq!(Program::load(&bytes), Ok(program));
    }

    #[test]
    fn encode_of_minimal_matches_the_hand_written_bytes() {
        let program = Program::load(&minimal()).unwrap();
        assert_eq!(program.encode(), minimal());
    }

    proptest! {
        /// Invariant 5 : aucune suite d'octets ne fait paniquer le chargeur.
        #[test]
        fn load_never_panics_on_arbitrary_bytes(bytes in prop::collection::vec(any::<u8>(), 0..300)) {
            let _ = Program::load(&bytes);
        }

        /// Un fichier valide dont on altère un octet ne fait pas paniquer non plus.
        #[test]
        fn load_never_panics_on_a_corrupted_valid_file(
            constants in prop::collection::vec(any::<i64>(), 0..8),
            code in prop::collection::vec(any::<u8>(), 1..32),
            index in any::<prop::sample::Index>(),
            value in any::<u8>(),
        ) {
            let mut bytes = Program::new(constants, code).unwrap().encode();
            let at = index.index(bytes.len());
            bytes[at] = value;
            let _ = Program::load(&bytes);
        }

        /// Aller-retour : `load(encode(p)) == p`.
        #[test]
        fn load_inverts_encode(
            constants in prop::collection::vec(any::<i64>(), 0..64),
            code in prop::collection::vec(any::<u8>(), 1..256),
        ) {
            let program = Program::new(constants, code).unwrap();
            prop_assert_eq!(Program::load(&program.encode()), Ok(program));
        }
    }
}
