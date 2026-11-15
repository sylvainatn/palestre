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
/// HALT, JUMPDEST, GAS et TRACE sont traités directement par le cycle.
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
                _ => execute(&mut self.state, instr, self.pc),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;

    fn program(code: &[u8]) -> Program {
        Program::new(vec![], code.to_vec()).unwrap()
    }
    fn budget(value: u16) -> NonZeroU16 {
        NonZeroU16::new(value).unwrap()
    }

    // Petit interpréteur de test : les instructions métier seront livrées séparément.
    fn execute(
        state: &mut State,
        instr: Instr,
        _: usize,
    ) -> Result<Control, ExecutionError<Infallible>> {
        match instr {
            Instr::PushI16(value) => state.stack.push(i64::from(value), Fault::StackOverflow)?,
            Instr::Drop => {
                state.stack.pop(Fault::StackUnderflow)?;
            }
            Instr::Store(addr) => {
                let value = state.stack.pop(Fault::StackUnderflow)?;
                state.memory[usize::from(addr)] = value;
            }
            _ => panic!("instruction inattendue : {instr:?}"),
        }
        Ok(Control::Next)
    }

    #[test]
    fn decoding_precedes_gas_and_execution() {
        for (code, fault, used) in [
            (vec![0x43], Fault::PcOutOfRange, 1),
            (vec![0x43, 0xff], Fault::BadOpcode, 1),
            (vec![0x43, 0x02], Fault::Truncated, 1),
            (vec![0x43, 0x03], Fault::OutOfGas, 1),
            (vec![0x03], Fault::StackUnderflow, 1),
        ] {
            let mut machine = Machine::default();
            let result = machine.run(&program(&code), budget(1), execute).unwrap();
            assert_eq!((result.fault, result.gas_used), (Some(fault), used));
        }
    }

    #[test]
    fn insufficient_gas_is_not_charged_and_does_not_call_executor() {
        let mut machine = Machine::default();
        let result = machine
            .run::<Infallible>(&program(&[0x13]), budget(6), |_, _, _| {
                panic!("instruction refusée exécutée")
            })
            .unwrap();
        assert_eq!(result.fault, Some(Fault::OutOfGas));
        assert_eq!(result.gas_used, 0);
    }

    #[test]
    fn halt_is_free_even_after_exhausting_budget() {
        let mut machine = Machine::default();
        let result = machine
            .run(&program(&[0x43, 0x00]), budget(1), execute)
            .unwrap();
        assert_eq!((result.fault, result.gas_used), (None, 1));
    }

    #[test]
    fn full_stack_without_gas_reports_out_of_gas() {
        let mut code = vec![];
        for _ in 0..65 {
            Instr::PushI16(1).encode(&mut code);
        }
        let mut machine = Machine::default();
        let p = program(&code);
        assert_eq!(
            machine.run(&p, budget(64), execute).unwrap().fault,
            Some(Fault::OutOfGas)
        );
        let result = machine.run(&p, budget(65), execute).unwrap();
        assert_eq!(
            (result.fault, result.gas_used),
            (Some(Fault::StackOverflow), 65)
        );
    }

    #[test]
    fn state_resets_but_memory_persists_and_trace_storage_is_reused() {
        let mut machine = Machine::default();
        let trace_ptr = machine.trace.as_ptr();
        let capacity = machine.trace.capacity();
        let p = program(&[0x02, 0, 42, 0x33, 7, 0x71, 0xf0, 0x00]);
        let result = machine.run(&p, budget(20), execute).unwrap();
        assert_eq!(result.memory[7], 42);
        assert_eq!(result.trace, &[13]);
        assert_eq!(result.gas_used, 8);
        machine
            .state
            .calls
            .push(12, Fault::CallStackOverflow)
            .unwrap();
        machine.state.stack.push(99, Fault::StackOverflow).unwrap();
        machine.state.action = Some(Action {
            kind: 0,
            arg: 1,
            ok: false,
        });
        let result = machine.run(&program(&[0x00]), budget(1), execute).unwrap();
        assert_eq!(result.memory[7], 42);
        assert!(result.trace.is_empty());
        assert_eq!(result.action, None);
        assert!(machine.state.stack.as_slice().is_empty());
        assert!(machine.state.calls.as_slice().is_empty());
        assert_eq!(machine.pc(), 0);
        assert_eq!(machine.gas_remaining(), 1);
        assert_eq!(machine.trace.as_ptr(), trace_ptr);
        assert_eq!(machine.trace.capacity(), capacity);
    }

    #[test]
    fn fault_preserves_prior_writes_and_trace() {
        let mut machine = Machine::default();
        let result = machine
            .run(
                &program(&[0x02, 0, 42, 0x33, 7, 0x71, 0xf0, 0x03]),
                budget(20),
                execute,
            )
            .unwrap();
        assert_eq!(result.fault, Some(Fault::StackUnderflow));
        assert_eq!(result.memory[7], 42);
        assert_eq!(result.trace, &[13]);
        assert_eq!(result.gas_used, 9);
    }

    #[test]
    fn technical_error_has_no_turn_result_and_next_run_resets() {
        let mut machine = Machine::default();
        let result = machine.run(&program(&[0x70]), budget(10), |_, _, _| {
            Err(ExecutionError::Environment("coupure"))
        });
        assert_eq!(result.unwrap_err(), "coupure");
        assert_eq!(machine.gas_remaining(), 5);
        assert!(machine
            .run(&program(&[0x00]), budget(1), execute)
            .unwrap()
            .fault
            .is_none());
    }

    #[test]
    fn fixed_stacks_enforce_limits_and_reuse_slots() {
        let mut stack = FixedStack::<usize, CALL_STACK_SIZE>::default();
        assert_eq!(
            stack.pop(Fault::CallStackUnderflow),
            Err(Fault::CallStackUnderflow)
        );
        for i in 0..CALL_STACK_SIZE {
            stack.push(i, Fault::CallStackOverflow).unwrap();
        }
        assert_eq!(
            stack.push(99, Fault::CallStackOverflow),
            Err(Fault::CallStackOverflow)
        );
        for i in (0..CALL_STACK_SIZE).rev() {
            assert_eq!(stack.pop(Fault::CallStackUnderflow), Ok(i));
        }
        stack.push(42, Fault::CallStackOverflow).unwrap();
        assert_eq!(stack.as_slice(), &[42]);
    }

    #[test]
    fn jumps_replace_normal_pc_advance() {
        let mut machine = Machine::default();
        let result = machine
            .run::<Infallible>(
                &program(&[0x40, 0, 1, 0xff, 0x43, 0x00]),
                budget(3),
                |_, instr, pc| {
                    assert_eq!((instr, pc), (Instr::Jmp(1), 0));
                    Ok(Control::Jump(4))
                },
            )
            .unwrap();
        assert_eq!((result.fault, result.gas_used), (None, 3));
    }
}
