//! Le chargeur contre le corpus public : chaque `program.pbc` est accepté ou refusé comme son
//! `case.toml` l'annonce, et le décodeur ne panique à aucun décalage d'aucun programme.
//!
//! Le harnais complet (exécution et comparaison des `expected.json`) viendra avec la machine.

use std::fs;
use std::path::{Path, PathBuf};

use palestre::fault::Fault;
use palestre::pbc::{decode, Program};

/// Tous les répertoires de cas, `edge/` puis `nominal/`.
fn case_dirs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let mut dirs = Vec::new();
    for nature in ["edge", "nominal"] {
        for entry in fs::read_dir(root.join(nature)).unwrap() {
            dirs.push(entry.unwrap().path());
        }
    }
    dirs.sort();
    dirs
}

/// Un cas de refus de chargement porte une clé `load_error` (FORMAT.md).
fn expects_refusal(dir: &Path) -> bool {
    let case = fs::read_to_string(dir.join("case.toml")).unwrap();
    case.lines()
        .any(|line| line.trim_start().starts_with("load_error"))
}

#[test]
fn every_corpus_program_loads_as_its_case_says() {
    let dirs = case_dirs();
    assert_eq!(dirs.len(), 65, "le corpus public compte 65 cas");

    for dir in &dirs {
        let bytes = fs::read(dir.join("program.pbc")).unwrap();
        let result = Program::load(&bytes);
        match (expects_refusal(dir), &result) {
            (true, Err(Fault::BadHeader)) | (false, Ok(_)) => {}
            (want, got) => panic!(
                "{} : refus attendu = {want}, obtenu = {got:?}",
                dir.display()
            ),
        }
    }
}

#[test]
fn every_loaded_corpus_program_survives_encode_and_decode_at_every_offset() {
    for dir in case_dirs().into_iter().filter(|d| !expects_refusal(d)) {
        let bytes = fs::read(dir.join("program.pbc")).unwrap();
        let program = Program::load(&bytes).unwrap();
        assert_eq!(program.encode(), bytes, "{}", dir.display());
        // Certains programmes contiennent exprès un opcode inconnu ou un opérande coupé :
        // on vérifie seulement que décoder n'importe où ne panique pas.
        for pc in 0..=program.code_len() {
            let _ = decode(program.code(), pc);
        }
    }
}
