//! Aides partagées par les tests d'intégration sur le corpus.

use std::fs;
use std::path::{Path, PathBuf};

/// Tous les répertoires de cas du corpus public, `edge/` puis `nominal/`, triés par nom.
pub fn case_dirs() -> Vec<PathBuf> {
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
