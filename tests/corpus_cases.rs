//! Le lecteur de `case.toml` contre le corpus public : chaque cas se lit, et compte autant
//! d'exécutions que son `expected.json`.

use std::fs;
use std::path::{Path, PathBuf};

use palestre::corpus::{Answer, Case};
use palestre::fault::Fault;

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

#[test]
fn every_public_case_is_read_and_agrees_with_its_expected_json() {
    let mut execs = 0;
    let mut refusals = 0;
    let mut answers = 0;

    for dir in case_dirs() {
        let case = Case::load(&dir.join("case.toml"))
            .unwrap_or_else(|err| panic!("{} : {err}", dir.display()));
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();

        // Le nom du répertoire est `NNN-catégorie-intitulé` : la catégorie doit concorder.
        let category = name.split('-').nth(1).unwrap();
        assert_eq!(case.category, category, "{name}");
        assert!(!case.description.is_empty(), "{name}");
        assert!(!case.clause.is_empty(), "{name}");

        match case.load_error {
            Some(fault) => {
                assert_eq!(fault, Fault::BadHeader, "{name}");
                assert!(case.execs.is_empty(), "{name}");
                assert!(!dir.join("expected.json").exists(), "{name}");
                refusals += 1;
            }
            None => {
                let expected: serde_json::Value =
                    serde_json::from_slice(&fs::read(dir.join("expected.json")).unwrap()).unwrap();
                let executions = expected["executions"].as_array().unwrap();
                assert_eq!(executions.len(), case.execs.len(), "{name}");
                for (exec, expected) in case.execs.iter().zip(executions) {
                    // Autant de réponses scriptées que de rappels attendus.
                    let calls = expected["calls"].as_array().unwrap();
                    assert_eq!(exec.answers.len(), calls.len(), "{name}");
                    for (answer, call) in exec.answers.iter().zip(calls) {
                        match answer {
                            Answer::Sense(v) => {
                                assert_eq!(call["call"], "SENSE", "{name}");
                                assert_eq!(call["value"], *v, "{name}");
                            }
                            Answer::SenseFault(f) => {
                                assert_eq!(call["call"], "SENSE", "{name}");
                                assert_eq!(call["fault"], f.as_str(), "{name}");
                            }
                            Answer::Act(ok) => {
                                assert_eq!(call["call"], "ACT", "{name}");
                                assert_eq!(call["ok"], *ok, "{name}");
                            }
                            Answer::Rand(v) => {
                                assert_eq!(call["call"], "RAND", "{name}");
                                assert_eq!(call["value"], *v, "{name}");
                            }
                        }
                    }
                    answers += exec.answers.len();
                }
                execs += case.execs.len();
            }
        }
    }

    // Les comptes de `corpus/INDEX.md` et du relevé fait sur les 65 cas.
    assert_eq!(refusals, 3);
    assert_eq!(execs, 68);
    assert_eq!(answers, 13);
}
