//! Format binaire `.pbc` — annexe A.
//!
//! Deux responsabilités, dans deux fichiers :
//!
//! * [`load`] — le **chargeur** : `&[u8]` → [`Program`] validé (magic `PBC1`, version `1`,
//!   `flags` nul, `const_count` dans `0..=4095`, `code_len` dans `1..=65535`, taille exacte du
//!   fichier). Toute violation est `BadHeader` (A.2), jamais un `panic` ni une lecture hors
//!   limites, et aucune allocation n'a lieu avant que les bornes soient vérifiées ;
//! * [`decode`] — le **décodeur** : à un décalage donné, lit un opcode et son opérande
//!   éventuel et rend l'[`Instr`] décodée, ou `PcOutOfRange` / `BadOpcode` / `Truncated`
//!   dans l'ordre des trois premières étapes de B.2.
//!
//! Le décodage est local : décoder à un décalage ne demande jamais de connaître ce qui le
//! précède (A.3). C'est ce qui autorise un saut au milieu d'un opérande.

mod decode;
mod load;

pub use decode::{decode, Instr, JUMPDEST};
pub use load::{
    Program, FORMAT_VERSION, HEADER_LEN, MAGIC, MAX_CODE_LEN, MAX_CONST_COUNT, MAX_FILE_LEN,
    MIN_CODE_LEN,
};
