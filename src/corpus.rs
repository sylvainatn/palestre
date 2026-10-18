//! Lecture des cas de test du corpus — `corpus/FORMAT.md`.
//!
//! Un cas est un répertoire `NNN-catégorie-intitulé/` avec `case.toml`, `program.pbc` et, sauf
//! refus de chargement, `expected.json`. Ce module lit le `case.toml` : la description, la
//! faute de chargement attendue s'il y en a une, et pour chaque `[[exec]]` le budget, la mémoire
//! initiale rendue dense sur 256 cellules et les réponses aux rappels.
//!
//! Le sujet interdit les générateurs de parseurs et n'autorise pas la crate `toml` (README §3) :
//! le lecteur est écrit à la main et ne reconnaît **que** le sous-ensemble de TOML que
//! `FORMAT.md` emploie (voir `journal/D-003`). Tout ce qui sort de ce sous-ensemble, et toute
//! clé inconnue, est une erreur : un cas qu'on ne sait pas lire ne doit pas être rejoué à moitié.
//!
//! Ce module ne sert qu'aux tests et aux outils de rejeu ; il n'a rien à voir avec la boucle
//! d'exécution, et peut allouer librement.

use std::fs;
use std::path::Path;

use crate::fault::Fault;
use crate::vm::MEM_SIZE;

/// Budget d'une exécution dont `case.toml` ne dit rien : `1000` (`FORMAT.md`, B.1).
pub const DEFAULT_BUDGET: u16 = 1000;

/// Un `case.toml` lu et validé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    /// Ce que le cas éprouve, en une phrase.
    pub description: String,
    /// La catégorie, la même que dans le nom du répertoire.
    pub category: String,
    /// La clause de `SPEC-PBC-v1.md` éprouvée.
    pub clause: String,
    /// Présent pour un refus de chargement : la faute que le chargeur doit lever (A.2).
    /// Le cas n'a alors aucune exécution.
    pub load_error: Option<Fault>,
    /// Les exécutions, dans l'ordre des `[[exec]]`, chacune indépendante des autres (K.9).
    pub execs: Vec<Exec>,
}

/// Une exécution : l'entrée d'un `EXEC` de K.3, et les réponses aux rappels qu'il déclenche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exec {
    /// Le `budget` de l'`EXEC`, dans `1..=65535`.
    pub budget: u16,
    /// La `mem` de l'`EXEC`, dense : les cellules absentes du `case.toml` valent `0`.
    pub mem: [i64; MEM_SIZE],
    /// La réponse à chaque rappel, dans l'ordre où la machine les pose.
    pub answers: Vec<Answer>,
}

impl Default for Exec {
    fn default() -> Self {
        Self {
            budget: DEFAULT_BUDGET,
            mem: [0; MEM_SIZE],
            answers: Vec::new(),
        }
    }
}

/// La réponse à un rappel (`FORMAT.md`, K.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// `sense = v` : un `SENSED` portant la valeur `v`.
    Sense(i64),
    /// `sense_fault = "BadSensor"` : un `SENSED` portant cette faute, que la machine lève.
    SenseFault(Fault),
    /// `act = true` / `act = false` : un `ACTED` portant cette issue.
    Act(bool),
    /// `rand = v` : un `DREW` portant la valeur `v`.
    Rand(i64),
}

/// Pourquoi un `case.toml` n'a pas pu être lu.
#[derive(Debug, thiserror::Error)]
pub enum CaseError {
    /// Le fichier n'a pas pu être lu.
    #[error("lecture de {path} : {source}")]
    Io {
        /// Le chemin demandé.
        path: String,
        /// L'erreur du système.
        source: std::io::Error,
    },
    /// Le texte sort du sous-ensemble de `FORMAT.md`, ou viole une de ses bornes.
    #[error("case.toml ligne {line} : {message}")]
    Syntax {
        /// Numéro de ligne, à partir de `1`.
        line: usize,
        /// Ce qui a été refusé.
        message: String,
    },
}

impl Case {
    /// Lit et valide le `case.toml` au chemin donné.
    pub fn load(path: &Path) -> Result<Self, CaseError> {
        let text = fs::read_to_string(path).map_err(|source| CaseError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Lit et valide le texte d'un `case.toml`.
    pub fn parse(text: &str) -> Result<Self, CaseError> {
        let mut case = Case {
            description: String::new(),
            category: String::new(),
            clause: String::new(),
            load_error: None,
            execs: Vec::new(),
        };
        // `None` tant qu'aucun `[[exec]]` n'a été vu : les clés vont alors au cas lui-même.
        let mut current: Option<Exec> = None;

        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let syntax = |message: String| CaseError::Syntax { line, message };
            let content = strip_comment(raw).trim();
            if content.is_empty() {
                continue;
            }
            if content == "[[exec]]" {
                if let Some(done) = current.take() {
                    case.execs.push(done);
                }
                current = Some(Exec::default());
                continue;
            }
            let (key, value) = parse_key_value(content).map_err(syntax)?;
            match current.as_mut() {
                None => case.set_field(&key, value).map_err(syntax)?,
                Some(exec) => exec.set_field(&key, value).map_err(syntax)?,
            }
        }
        if let Some(done) = current.take() {
            case.execs.push(done);
        }

        if case.load_error.is_some() && !case.execs.is_empty() {
            return Err(CaseError::Syntax {
                line: 0,
                message: "un cas de refus de chargement n'a aucune exécution".to_string(),
            });
        }
        Ok(case)
    }

    fn set_field(&mut self, key: &str, value: Value) -> Result<(), String> {
        match key {
            "description" => self.description = value.into_string(key)?,
            "category" => self.category = value.into_string(key)?,
            "clause" => self.clause = value.into_string(key)?,
            "load_error" => {
                let name = value.into_string(key)?;
                let fault = Fault::ALL
                    .into_iter()
                    .find(|fault| fault.as_str() == name)
                    .ok_or_else(|| format!("`load_error` : faute inconnue `{name}`"))?;
                self.load_error = Some(fault);
            }
            other => return Err(format!("clé inconnue hors d'un `[[exec]]` : `{other}`")),
        }
        Ok(())
    }
}

impl Exec {
    fn set_field(&mut self, key: &str, value: Value) -> Result<(), String> {
        match key {
            "budget" => {
                let budget = value.into_int(key)?;
                self.budget = u16::try_from(budget)
                    .ok()
                    .filter(|b| *b >= 1)
                    .ok_or_else(|| format!("`budget` doit être dans 1..=65535, pas {budget}"))?;
            }
            "mem" => {
                for (cell, value) in value.into_table(key)? {
                    let index = cell
                        .parse::<usize>()
                        .ok()
                        .filter(|i| *i < MEM_SIZE)
                        .ok_or_else(|| format!("`mem` : cellule `{cell}` hors de 0..=255"))?;
                    let slot = self
                        .mem
                        .get_mut(index)
                        .ok_or_else(|| format!("`mem` : cellule `{cell}` hors de 0..=255"))?;
                    *slot = value.into_int("mem")?;
                }
            }
            "answers" => {
                for item in value.into_array(key)? {
                    self.answers
                        .push(Answer::from_table(item.into_table("answers")?)?);
                }
            }
            other => return Err(format!("clé inconnue dans un `[[exec]]` : `{other}`")),
        }
        Ok(())
    }
}

impl Answer {
    /// Une réponse porte exactement **une** des quatre clés de `FORMAT.md`.
    fn from_table(mut fields: Vec<(String, Value)>) -> Result<Self, String> {
        let (key, value) = match (fields.pop(), fields.is_empty()) {
            (Some(single), true) => single,
            _ => return Err("une réponse porte exactement une clé".to_string()),
        };
        match key.as_str() {
            "sense" => Ok(Answer::Sense(value.into_int(&key)?)),
            "sense_fault" => {
                let name = value.into_string(&key)?;
                Fault::ALL
                    .into_iter()
                    .find(|fault| fault.as_str() == name)
                    .map(Answer::SenseFault)
                    .ok_or_else(|| format!("`sense_fault` : faute inconnue `{name}`"))
            }
            "act" => Ok(Answer::Act(value.into_bool(&key)?)),
            "rand" => Ok(Answer::Rand(value.into_int(&key)?)),
            other => Err(format!("réponse inconnue `{other}`")),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Le sous-ensemble de TOML de FORMAT.md : une ligne `clé = valeur`, où une valeur est une chaîne
// `"…"`, un entier, `true`/`false`, une table en ligne `{ clé = valeur, … }` ou un tableau
// `[ valeur, … ]`.
// ---------------------------------------------------------------------------------------------

/// Une valeur TOML du sous-ensemble reconnu.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Str(String),
    Int(i64),
    Bool(bool),
    Table(Vec<(String, Value)>),
    Array(Vec<Value>),
}

impl Value {
    fn kind(&self) -> &'static str {
        match self {
            Value::Str(_) => "une chaîne",
            Value::Int(_) => "un entier",
            Value::Bool(_) => "un booléen",
            Value::Table(_) => "une table",
            Value::Array(_) => "un tableau",
        }
    }

    fn into_string(self, key: &str) -> Result<String, String> {
        match self {
            Value::Str(s) => Ok(s),
            other => Err(format!("`{key}` attend une chaîne, pas {}", other.kind())),
        }
    }

    fn into_int(self, key: &str) -> Result<i64, String> {
        match self {
            Value::Int(i) => Ok(i),
            other => Err(format!("`{key}` attend un entier, pas {}", other.kind())),
        }
    }

    fn into_bool(self, key: &str) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(b),
            other => Err(format!("`{key}` attend un booléen, pas {}", other.kind())),
        }
    }

    fn into_table(self, key: &str) -> Result<Vec<(String, Value)>, String> {
        match self {
            Value::Table(t) => Ok(t),
            other => Err(format!("`{key}` attend une table, pas {}", other.kind())),
        }
    }

    fn into_array(self, key: &str) -> Result<Vec<Value>, String> {
        match self {
            Value::Array(a) => Ok(a),
            other => Err(format!("`{key}` attend un tableau, pas {}", other.kind())),
        }
    }
}

/// Retire un commentaire `# …` qui n'est pas à l'intérieur d'une chaîne.
fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut bytes = line.bytes().enumerate();
    while let Some((at, byte)) = bytes.next() {
        match byte {
            b'"' => in_string = !in_string,
            // Dans une chaîne, l'octet qui suit un antislash est échappé : on le saute,
            // pour qu'un `\"` ne ferme pas la chaîne.
            b'\\' if in_string => {
                bytes.next();
            }
            b'#' if !in_string => return line.get(..at).unwrap_or(line),
            _ => {}
        }
    }
    line
}

/// Lit une ligne `clé = valeur` entière ; tout reste est une erreur.
fn parse_key_value(content: &str) -> Result<(String, Value), String> {
    let mut scanner = Scanner::new(content);
    let key = scanner.key()?;
    scanner.skip_spaces();
    scanner.expect(b'=')?;
    scanner.skip_spaces();
    let value = scanner.value()?;
    scanner.skip_spaces();
    if !scanner.at_end() {
        return Err(format!(
            "texte inattendu après la valeur : `{}`",
            scanner.rest()
        ));
    }
    Ok((key, value))
}

/// Un curseur sur une ligne. Toute lecture passe par `get`, jamais par une indexation.
struct Scanner<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.src.as_bytes().get(self.pos).copied()
    }

    fn at_end(&self) -> bool {
        self.peek().is_none()
    }

    fn rest(&self) -> &'a str {
        self.src.get(self.pos..).unwrap_or("")
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.bump();
        }
    }

    fn expect(&mut self, wanted: u8) -> Result<(), String> {
        match self.peek() {
            Some(found) if found == wanted => {
                self.bump();
                Ok(())
            }
            Some(found) => Err(format!(
                "`{}` attendu, trouvé `{}`",
                wanted as char, found as char
            )),
            None => Err(format!(
                "`{}` attendu, fin de ligne trouvée",
                wanted as char
            )),
        }
    }

    /// Une clé nue : lettres, chiffres, `_` et `-` (TOML « bare key »). `3` en est une.
    fn key(&mut self) -> Result<String, String> {
        let start = self.pos;
        while matches!(
            self.peek(),
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-')
        ) {
            self.bump();
        }
        let key = self.src.get(start..self.pos).unwrap_or("");
        if key.is_empty() {
            return Err("clé attendue".to_string());
        }
        Ok(key.to_string())
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some(b'"') => self.string().map(Value::Str),
            Some(b'{') => self.table().map(Value::Table),
            Some(b'[') => self.array().map(Value::Array),
            Some(b'-' | b'0'..=b'9') => self.integer().map(Value::Int),
            Some(b't' | b'f') => self.boolean().map(Value::Bool),
            Some(other) => Err(format!("valeur attendue, trouvé `{}`", other as char)),
            None => Err("valeur attendue, fin de ligne trouvée".to_string()),
        }
    }

    /// Une chaîne basique `"…"` avec les échappements `\"`, `\\`, `\n`, `\t`.
    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let start = self.pos;
            while !matches!(self.peek(), Some(b'"' | b'\\') | None) {
                self.bump();
            }
            // `start` et `pos` sont tous deux sur un octet ASCII ou en fin de ligne : la coupe
            // tombe sur une frontière de caractère, et le texte entre les deux est copié tel quel.
            out.push_str(self.src.get(start..self.pos).unwrap_or(""));
            match self.peek() {
                Some(b'"') => {
                    self.bump();
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.bump();
                    let escaped = match self.peek() {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'n') => '\n',
                        Some(b't') => '\t',
                        other => {
                            return Err(format!(
                                "échappement inconnu `\\{}`",
                                other.map(|b| b as char).unwrap_or(' ')
                            ))
                        }
                    };
                    self.bump();
                    out.push(escaped);
                }
                _ => return Err("chaîne non terminée".to_string()),
            }
        }
    }

    /// Un entier décimal signé, avec `_` toléré entre les chiffres comme en TOML.
    fn integer(&mut self) -> Result<i64, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        while matches!(self.peek(), Some(b'0'..=b'9' | b'_')) {
            self.bump();
        }
        let text = self.src.get(start..self.pos).unwrap_or("");
        let digits: String = text.chars().filter(|c| *c != '_').collect();
        digits
            .parse::<i64>()
            .map_err(|_| format!("entier invalide `{text}`"))
    }

    fn boolean(&mut self) -> Result<bool, String> {
        for (word, value) in [("true", true), ("false", false)] {
            if self.rest().starts_with(word) {
                self.pos += word.len();
                return Ok(value);
            }
        }
        Err(format!(
            "`true` ou `false` attendu, trouvé `{}`",
            self.rest()
        ))
    }

    /// `{ clé = valeur, … }`, éventuellement vide.
    fn table(&mut self) -> Result<Vec<(String, Value)>, String> {
        self.expect(b'{')?;
        let mut fields = Vec::new();
        self.skip_spaces();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(fields);
        }
        loop {
            self.skip_spaces();
            let key = self.key()?;
            self.skip_spaces();
            self.expect(b'=')?;
            self.skip_spaces();
            let value = self.value()?;
            fields.push((key, value));
            self.skip_spaces();
            match self.peek() {
                Some(b',') => self.bump(),
                Some(b'}') => {
                    self.bump();
                    return Ok(fields);
                }
                _ => return Err("`,` ou `}` attendu dans une table".to_string()),
            }
        }
    }

    /// `[ valeur, … ]`, éventuellement vide, virgule finale tolérée comme en TOML.
    fn array(&mut self) -> Result<Vec<Value>, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        loop {
            self.skip_spaces();
            if self.peek() == Some(b']') {
                self.bump();
                return Ok(items);
            }
            items.push(self.value()?);
            self.skip_spaces();
            match self.peek() {
                Some(b',') => self.bump(),
                Some(b']') => {
                    self.bump();
                    return Ok(items);
                }
                _ => return Err("`,` ou `]` attendu dans un tableau".to_string()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Case {
        Case::parse(text).unwrap()
    }

    fn error(text: &str) -> String {
        Case::parse(text).unwrap_err().to_string()
    }

    #[test]
    fn the_example_of_format_md() {
        let case = parse(
            r#"description = "an execution whose remaining gas is exactly zero"
category = "gas"
clause = "B.2"

[[exec]]
budget = 12                 # budget de l'exécution, de 1 à 65535 ; 1000 s'il est omis
mem = { 3 = 42 }            # mémoire initiale creuse : cellule (0 à 255) → valeur
answers = [{ sense = 0 }]   # réponses aux rappels, dans l'ordre où la machine les pose
"#,
        );
        assert_eq!(
            case.description,
            "an execution whose remaining gas is exactly zero"
        );
        assert_eq!(case.category, "gas");
        assert_eq!(case.clause, "B.2");
        assert_eq!(case.load_error, None);
        assert_eq!(case.execs.len(), 1);
        let exec = &case.execs[0];
        assert_eq!(exec.budget, 12);
        assert_eq!(exec.mem[3], 42);
        assert_eq!(exec.mem.iter().filter(|v| **v != 0).count(), 1);
        assert_eq!(exec.answers, vec![Answer::Sense(0)]);
    }

    #[test]
    fn a_load_error_case_has_no_exec() {
        let case = parse("description = \"x\"\nload_error = \"BadHeader\"\n");
        assert_eq!(case.load_error, Some(Fault::BadHeader));
        assert!(case.execs.is_empty());
    }

    #[test]
    fn a_load_error_with_an_exec_is_refused() {
        let message = error("load_error = \"BadHeader\"\n[[exec]]\n");
        assert!(message.contains("aucune exécution"), "{message}");
    }

    #[test]
    fn an_unknown_load_error_is_refused() {
        assert!(error("load_error = \"BadMagic\"").contains("faute inconnue"));
    }

    #[test]
    fn an_empty_exec_takes_the_defaults() {
        let case = parse("[[exec]]\n");
        assert_eq!(case.execs, vec![Exec::default()]);
        assert_eq!(Exec::default().budget, 1000);
    }

    #[test]
    fn several_execs_keep_their_order_and_their_own_fields() {
        let case = parse(
            "[[exec]]\nanswers = [{ sense = 1 }]\n[[exec]]\n[[exec]]\nbudget = 7\nmem = { 0 = 42, 18 = 9 }\n",
        );
        assert_eq!(case.execs.len(), 3);
        assert_eq!(case.execs[0].answers, vec![Answer::Sense(1)]);
        assert_eq!(case.execs[1], Exec::default());
        assert_eq!(case.execs[2].budget, 7);
        assert_eq!(case.execs[2].mem[0], 42);
        assert_eq!(case.execs[2].mem[18], 9);
        assert!(case.execs[2].answers.is_empty());
    }

    #[test]
    fn all_four_answer_kinds() {
        let case = parse(
            "[[exec]]\nanswers = [{ sense = -3 }, { sense_fault = \"BadSensor\" }, { act = false }, { rand = 42 }]\n",
        );
        assert_eq!(
            case.execs[0].answers,
            vec![
                Answer::Sense(-3),
                Answer::SenseFault(Fault::BadSensor),
                Answer::Act(false),
                Answer::Rand(42),
            ]
        );
    }

    #[test]
    fn an_answer_has_exactly_one_key() {
        assert!(error("[[exec]]\nanswers = [{ sense = 1, act = true }]").contains("exactement une"));
        assert!(error("[[exec]]\nanswers = [{ }]").contains("exactement une"));
        assert!(error("[[exec]]\nanswers = [{ feel = 1 }]").contains("réponse inconnue"));
    }

    #[test]
    fn budget_bounds() {
        assert_eq!(parse("[[exec]]\nbudget = 65535").execs[0].budget, 65535);
        assert_eq!(parse("[[exec]]\nbudget = 1").execs[0].budget, 1);
        assert!(error("[[exec]]\nbudget = 0").contains("1..=65535"));
        assert!(error("[[exec]]\nbudget = 65536").contains("1..=65535"));
        assert!(error("[[exec]]\nbudget = -1").contains("1..=65535"));
    }

    #[test]
    fn mem_bounds_and_extremes() {
        let case = parse("[[exec]]\nmem = { 0 = -9223372036854775808, 255 = 9223372036854775807 }");
        assert_eq!(case.execs[0].mem[0], i64::MIN);
        assert_eq!(case.execs[0].mem[255], i64::MAX);
        assert!(error("[[exec]]\nmem = { 256 = 1 }").contains("hors de 0..=255"));
        assert!(error("[[exec]]\nmem = { -1 = 1 }").contains("hors de 0..=255"));
        assert!(error("[[exec]]\nmem = { 1 = \"x\" }").contains("attend un entier"));
    }

    #[test]
    fn unknown_keys_are_refused_so_that_a_hidden_case_is_never_half_replayed() {
        assert!(error("author = \"x\"").contains("clé inconnue"));
        assert!(error("[[exec]]\nbudgett = 3").contains("clé inconnue"));
        assert!(error("[[exec]]\ndescription = \"x\"").contains("clé inconnue"));
    }

    #[test]
    fn comments_and_blank_lines_are_ignored_but_not_inside_strings() {
        let case = parse("# en-tête\n\ndescription = \"a # b\" # fin\n   \n[[exec]] \n");
        assert_eq!(case.description, "a # b");
        assert_eq!(case.execs.len(), 1);
    }

    #[test]
    fn string_escapes_and_unicode() {
        let case = parse(r#"description = "dit \"ça\" \\ é→ß""#);
        assert_eq!(case.description, "dit \"ça\" \\ é→ß");
        assert!(error(r#"description = "ouverte"#).contains("non terminée"));
        assert!(error(r#"description = "\q""#).contains("échappement"));
    }

    #[test]
    fn syntax_errors_name_the_line() {
        let err = Case::parse("description = \"ok\"\n[[exec]]\nbudget 12\n").unwrap_err();
        assert!(matches!(err, CaseError::Syntax { line: 3, .. }), "{err}");
        assert!(error("[[exec]]\nbudget = 12 extra").contains("inattendu"));
        assert!(error("[[exec]]\nanswers = [{ sense = 1 }").contains("tableau"));
        assert!(error("[[exec]]\nmem = { 1 = 2").contains("table"));
        assert!(error("= 3").contains("clé attendue"));
    }

    #[test]
    fn integers_accept_underscores_like_toml() {
        assert_eq!(parse("[[exec]]\nbudget = 1_000").execs[0].budget, 1000);
    }
}
