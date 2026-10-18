//! Le chargeur contre le corpus public : chaque `program.pbc` est accepté ou refusé comme son
//! `case.toml` l'annonce, et le décodeur ne panique à aucun décalage d'aucun programme.
//!
//! Le harnais complet (exécution et comparaison des `expected.json`) viendra avec la machine.

mod common;

use std::fs;

use common::case_dirs;
use palestre::corpus::Case;
use palestre::fault::Fault;
use palestre::pbc::{decode, Program};

#[test]
fn every_corpus_program_loads_as_its_case_says() {
    let dirs = case_dirs();
    assert_eq!(dirs.len(), 65, "le corpus public compte 65 cas");

    for dir in &dirs {
        let case = Case::load(&dir.join("case.toml")).unwrap();
        let bytes = fs::read(dir.join("program.pbc")).unwrap();
        let result = Program::load(&bytes);
        match (case.load_error, &result) {
            (Some(Fault::BadHeader), Err(Fault::BadHeader)) | (None, Ok(_)) => {}
            (want, got) => panic!(
                "{} : refus attendu = {want:?}, obtenu = {got:?}",
                dir.display()
            ),
        }
    }
}

#[test]
fn every_loaded_corpus_program_survives_encode_and_decode_at_every_offset() {
    for dir in case_dirs() {
        let bytes = fs::read(dir.join("program.pbc")).unwrap();
        let Ok(program) = Program::load(&bytes) else {
            continue;
        };
        assert_eq!(program.encode(), bytes, "{}", dir.display());
        // Certains programmes contiennent exprès un opcode inconnu ou un opérande coupé :
        // on vérifie seulement que décoder n'importe où ne panique pas.
        for pc in 0..=program.code_len() {
            let _ = decode(program.code(), pc);
        }
    }
}
