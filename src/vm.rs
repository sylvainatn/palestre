//! Machine virtuelle — annexes B et D.
//!
//! Responsabilités de ce module :
//!
//! * les ressources de B.1 en tableaux de taille fixe : pile de 64 `i64`, pile d'appels de
//!   16 adresses, mémoire de 256 cellules, budget de gaz `u16` ;
//! * le cycle d'exécution de B.2, dans l'ordre normatif : `PcOutOfRange`, décodage, `Truncated`,
//!   **prélèvement du gaz**, exécution, avance du `pc` ;
//! * la sémantique des 43 instructions de B.3, les sauts de B.4, les décalages de B.6, `GAS` de B.7 ;
//! * la liste **close** des seize fautes de B.5, dont l'affichage rend la chaîne exacte de la spec ;
//! * la table de gaz de l'annexe D, écrite à **un seul endroit**.
//!
//! La machine ne connaît aucun monde : `SENSE`, `ACT` et `RAND` sont délégués à un
//! environnement fourni par l'appelant (arène locale, serveur réseau ou harnais de test).
//! Aucune allocation n'a lieu pendant un tour (README §3).

/// Nombre de cellules de la mémoire d'un agent, indices `0..=255` (B.1).
pub const MEM_SIZE: usize = 256;

use std::num::NonZeroU16;

use crate::{
    fault::Fault,
    pbc::{decode, Instr, Program},
};

/// Capacité de la pile de données (B.1).
pub const STACK_SIZE: usize = 64;
/// Capacité de la pile d'appels (B.1).
pub const CALL_STACK_SIZE: usize = 16;

/// Pile bornée sans allocation. Les cases inutilisées ne font pas partie de son état logique.
#[derive(Debug)]
pub struct FixedStack<T, const N: usize> {
    cells: [T; N],
    len: usize,
}

impl<T: Copy + Default, const N: usize> Default for FixedStack<T, N> {
    fn default() -> Self {
        Self {
            cells: [T::default(); N],
            len: 0,
        }
    }
}

impl<T: Copy, const N: usize> FixedStack<T, N> {
    /// Valeurs occupées, du fond vers le sommet.
    pub fn as_slice(&self) -> &[T] {
        self.cells.get(..self.len).unwrap_or_default()
    }

    /// Vide la pile sans allocation.
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Empile, ou renvoie la faute fournie si la pile est pleine.
    pub fn push(&mut self, value: T, overflow: Fault) -> Result<(), Fault> {
        let cell = self.cells.get_mut(self.len).ok_or(overflow)?;
        *cell = value;
        self.len += 1;
        Ok(())
    }

    /// Dépile, ou renvoie la faute fournie si la pile est vide.
    pub fn pop(&mut self, underflow: Fault) -> Result<T, Fault> {
        let next = self.len.checked_sub(1).ok_or(underflow)?;
        let value = self.cells.get(next).copied().ok_or(underflow)?;
        self.len = next;
        Ok(value)
    }
}

/// État accessible à la sémantique des instructions ; le cycle contrôle seul le pc et le gaz.
#[derive(Debug)]
pub struct State {
    /// Pile de données, vide au début du tour.
    pub stack: FixedStack<i64, STACK_SIZE>,
    /// Adresses de retour, vides au début du tour.
    pub calls: FixedStack<usize, CALL_STACK_SIZE>,
    /// Mémoire persistante ; aucune restauration automatique sur faute dans la VM.
    pub memory: [i64; MEM_SIZE],
    /// Tentative d'action de ce tour, conservée même si une faute survient ensuite.
    pub action: Option<Action>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            stack: FixedStack::default(),
            calls: FixedStack::default(),
            memory: [0; MEM_SIZE],
            action: None,
        }
    }
}

/// Action validée et répondue par l'environnement (E.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    /// Numéro de l'action.
    pub kind: u8,
    /// Argument dépilé.
    pub arg: i64,
    /// Réussite ou échec ordinaire.
    pub ok: bool,
}

/// Suite du cycle après l'effet d'une instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// Instruction ordinaire : avancer de sa taille.
    Next,
    /// Destination déjà validée par la sémantique des sauts (B.4).
    Jump(usize),
}

/// Une faute normative termine le tour ; une erreur technique interrompt sans résultat (K.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError<E> {
    /// Faute du tour.
    Fault(Fault),
    /// Incident de l'environnement ; jamais converti en faute normative.
    Environment(E),
}

impl<E> From<Fault> for ExecutionError<E> {
    fn from(fault: Fault) -> Self {
        Self::Fault(fault)
    }
}

/// Résultat emprunté : aucune copie allouée de la trace à la fin du tour.
#[derive(Debug)]
pub struct TurnResult<'a> {
    /// Mémoire après exécution, y compris les écritures précédant une faute.
    pub memory: &'a [i64; MEM_SIZE],
    /// Valeurs émises par TRACE pendant ce tour.
    pub trace: &'a [i64],
    /// Gaz effectivement prélevé.
    pub gas_used: u16,
    /// Absente pour HALT, présente pour une faute normative.
    pub fault: Option<Fault>,
    /// Tentative d'action du tour.
    pub action: Option<Action>,
}

/// Coût normatif de chaque instruction (B.3/D), défini à un seul endroit.
pub const fn gas_cost(instr: Instr) -> u16 {
    match instr {
        Instr::Halt => 0,
        Instr::Const(_)
        | Instr::PushI16(_)
        | Instr::Drop
        | Instr::Dup
        | Instr::Swap
        | Instr::Over
        | Instr::Rot
        | Instr::JumpDest
        | Instr::Trace => 1,
        Instr::ConstX
        | Instr::Add
        | Instr::Sub
        | Instr::Mul
        | Instr::Neg
        | Instr::Eq
        | Instr::Lt
        | Instr::Gt
        | Instr::Not
        | Instr::And
        | Instr::Or
        | Instr::BNot
        | Instr::Jmp(_)
        | Instr::Jz(_)
        | Instr::Jnz(_)
        | Instr::Gas => 2,
        Instr::Shl | Instr::Load(_) | Instr::JmpX => 3,
        Instr::Xor | Instr::LoadX | Instr::Store(_) | Instr::Return => 4,
        Instr::StoreX | Instr::Rand => 5,
        Instr::Shr | Instr::Sar | Instr::Call(_) => 6,
        Instr::Div | Instr::Mod | Instr::CallX => 7,
        Instr::Sense(_) => 8,
        Instr::Act(_) => 12,
    }
}

/// Socle B.1–B.2. La sémantique métier des instructions est injectée dans [`Machine::run`].
/// Les instructions pures de B.3, HALT, JUMPDEST, GAS et TRACE sont intégrées.
#[derive(Debug)]
pub struct Machine {
    state: State,
    pc: usize,
    gas_remaining: u16,
    trace: Vec<i64>,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new([0; MEM_SIZE])
    }
}

impl Machine {
    /// Installe la mémoire et réserve la trace pour tout budget u16 avant tout tour.
    /// Cette unique allocation est réutilisée, même si le budget change entre deux tours.
    pub fn new(memory: [i64; MEM_SIZE]) -> Self {
        Self {
            state: State {
                memory,
                ..State::default()
            },
            pc: 0,
            gas_remaining: 0,
            trace: Vec::with_capacity(usize::from(u16::MAX)),
        }
    }

    /// État courant, également consultable après une interruption technique.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Adresse de l'instruction courante (ou suivante après une avance normale).
    pub fn pc(&self) -> usize {
        self.pc
    }

    /// Gaz restant, après prélèvement du coût de l'instruction courante.
    pub fn gas_remaining(&self) -> u16 {
        self.gas_remaining
    }

    /// Exécute un tour avec un budget non nul. Aucun tampon n'est alloué dans cette méthode.
    ///
    /// Les constantes, opérations de pile, calculs et comparaisons sont intégrés au cycle.
    /// `execute` reçoit l'état, l'instruction et son adresse. Il doit appliquer B.3/B.4,
    /// sans allocation, et renvoyer Next ou une destination validée. Les rappels et le
    /// contrôles d'ACT appartiennent à cette sémantique ; State::action est vidé ici.
    /// Une faute est rendue dans TurnResult ; une erreur technique est propagée sans résultat.
    pub fn run<E>(
        &mut self,
        program: &Program,
        budget: NonZeroU16,
        mut execute: impl FnMut(&mut State, Instr, usize) -> Result<Control, ExecutionError<E>>,
    ) -> Result<TurnResult<'_>, E> {
        self.state.stack.clear();
        self.state.calls.clear();
        self.state.action = None;
        self.pc = 0;
        self.gas_remaining = budget.get();
        self.trace.clear();

        let fault = loop {
            // decode applique les étapes 1–3 dans l'ordre, sans prélèvement de gaz.
            let instr = match decode(program.code(), self.pc) {
                Ok(instr) => instr,
                Err(fault) => break Some(fault),
            };
            let Some(remaining) = self.gas_remaining.checked_sub(gas_cost(instr)) else {
                break Some(Fault::OutOfGas);
            };
            self.gas_remaining = remaining;
            let effect = match instr {
                Instr::Halt => break None,
                Instr::JumpDest => Ok(Control::Next),
                Instr::Gas => self
                    .state
                    .stack
                    .push(i64::from(remaining), Fault::StackOverflow)
                    .map(|()| Control::Next)
                    .map_err(ExecutionError::Fault),
                Instr::Trace => self
                    .state
                    .stack
                    .pop(Fault::StackUnderflow)
                    .map(|value| {
                        // Chaque TRACE payé coûte 1 : au plus budget <= 65535 ajouts.
                        self.trace.push(value);
                        Control::Next
                    })
                    .map_err(ExecutionError::Fault),
                _ => match execute_core(&mut self.state.stack, program, instr) {
                    Some(result) => result
                        .map(|()| Control::Next)
                        .map_err(ExecutionError::Fault),
                    None => execute(&mut self.state, instr, self.pc),
                },
            };
            match effect {
                Ok(Control::Next) => self.pc += instr.size(),
                Ok(Control::Jump(target)) => self.pc = target,
                Err(ExecutionError::Fault(fault)) => break Some(fault),
                Err(ExecutionError::Environment(error)) => return Err(error),
            }
        };
        Ok(TurnResult {
            memory: &self.state.memory,
            trace: &self.trace,
            gas_used: budget.get() - self.gas_remaining,
            fault,
            action: self.state.action,
        })
    }
}

// None réserve les instructions hors de ce ticket à la fonction injectée.
fn execute_core(
    stack: &mut FixedStack<i64, STACK_SIZE>,
    program: &Program,
    instr: Instr,
) -> Option<Result<(), Fault>> {
    let mut result = || {
        let value = match instr {
            Instr::Const(index) => *program
                .constants()
                .get(usize::from(index))
                .ok_or(Fault::BadConst)?,
            Instr::ConstX => {
                let index = stack.pop(Fault::StackUnderflow)?;
                let index = usize::try_from(index).map_err(|_| Fault::BadConst)?;
                *program.constants().get(index).ok_or(Fault::BadConst)?
            }
            Instr::PushI16(value) => i64::from(value),
            Instr::Drop => {
                stack.pop(Fault::StackUnderflow)?;
                return Ok(());
            }
            Instr::Dup => {
                let a = stack.pop(Fault::StackUnderflow)?;
                stack.push(a, Fault::StackOverflow)?;
                a
            }
            Instr::Swap | Instr::Over => {
                let b = stack.pop(Fault::StackUnderflow)?;
                let a = stack.pop(Fault::StackUnderflow)?;
                if instr == Instr::Swap {
                    stack.push(b, Fault::StackOverflow)?;
                    a
                } else {
                    stack.push(a, Fault::StackOverflow)?;
                    stack.push(b, Fault::StackOverflow)?;
                    a
                }
            }
            Instr::Rot => {
                let c = stack.pop(Fault::StackUnderflow)?;
                let b = stack.pop(Fault::StackUnderflow)?;
                let a = stack.pop(Fault::StackUnderflow)?;
                stack.push(b, Fault::StackOverflow)?;
                stack.push(c, Fault::StackOverflow)?;
                a
            }
            Instr::Neg | Instr::Not | Instr::BNot => {
                let a = stack.pop(Fault::StackUnderflow)?;
                match instr {
                    Instr::Neg => a.wrapping_neg(),
                    Instr::Not => i64::from(a == 0),
                    _ => !a,
                }
            }
            Instr::Add
            | Instr::Sub
            | Instr::Mul
            | Instr::Div
            | Instr::Mod
            | Instr::Eq
            | Instr::Lt
            | Instr::Gt
            | Instr::And
            | Instr::Or
            | Instr::Xor
            | Instr::Shl
            | Instr::Shr
            | Instr::Sar => {
                let b = stack.pop(Fault::StackUnderflow)?;
                let a = stack.pop(Fault::StackUnderflow)?;
                match instr {
                    Instr::Add => a.wrapping_add(b),
                    Instr::Sub => a.wrapping_sub(b),
                    Instr::Mul => a.wrapping_mul(b),
                    Instr::Div | Instr::Mod => {
                        if b == 0 {
                            return Err(Fault::DivByZero);
                        }
                        if instr == Instr::Div {
                            a.wrapping_div(b)
                        } else {
                            a.wrapping_rem(b)
                        }
                    }
                    Instr::Eq => i64::from(a == b),
                    Instr::Lt => i64::from(a < b),
                    Instr::Gt => i64::from(a > b),
                    Instr::And => a & b,
                    Instr::Or => a | b,
                    Instr::Xor => a ^ b,
                    // Vérifier le compte avant le décalage : Rust masque autrement les grands comptes.
                    _ if !(0..=63).contains(&b) => {
                        if instr == Instr::Sar && a < 0 {
                            -1
                        } else {
                            0
                        }
                    }
                    Instr::Shl => ((a as u64) << b) as i64,
                    Instr::Shr => ((a as u64) >> b) as i64,
                    _ => a >> b,
                }
            }
            _ => return Ok(()),
        };
        stack.push(value, Fault::StackOverflow)
    };
    match instr {
        Instr::Const(_)
        | Instr::ConstX
        | Instr::PushI16(_)
        | Instr::Drop
        | Instr::Dup
        | Instr::Swap
        | Instr::Over
        | Instr::Rot
        | Instr::Neg
        | Instr::Not
        | Instr::BNot
        | Instr::Add
        | Instr::Sub
        | Instr::Mul
        | Instr::Div
        | Instr::Mod
        | Instr::Eq
        | Instr::Lt
        | Instr::Gt
        | Instr::And
        | Instr::Or
        | Instr::Xor
        | Instr::Shl
        | Instr::Shr
        | Instr::Sar => Some(result()),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
