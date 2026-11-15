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
fn normative_costs_and_budget_boundaries_for_every_opcode() {
    // Oracle indépendant de B.3/D : détecte notamment les coûts non monotones.
    let costs = [
        (0x00, 0),
        (0x01, 1),
        (0x02, 1),
        (0x03, 1),
        (0x04, 1),
        (0x05, 1),
        (0x06, 1),
        (0x07, 1),
        (0x08, 2),
        (0x10, 2),
        (0x11, 2),
        (0x12, 2),
        (0x13, 7),
        (0x14, 7),
        (0x15, 2),
        (0x20, 2),
        (0x21, 2),
        (0x22, 2),
        (0x23, 2),
        (0x24, 2),
        (0x25, 2),
        (0x26, 4),
        (0x27, 3),
        (0x28, 6),
        (0x29, 2),
        (0x2d, 6),
        (0x30, 4),
        (0x31, 5),
        (0x32, 3),
        (0x33, 4),
        (0x40, 2),
        (0x41, 2),
        (0x42, 2),
        (0x43, 1),
        (0x44, 3),
        (0x50, 6),
        (0x51, 4),
        (0x52, 7),
        (0x60, 8),
        (0x61, 12),
        (0x70, 5),
        (0x71, 2),
        (0xf0, 1),
    ];
    let mut machine = Machine::default();
    for (opcode, cost) in costs {
        let instr = decode(&[opcode, 0, 0], 0).unwrap();
        assert_eq!(gas_cost(instr), cost, "{instr:?}");
        let mut code = Vec::new();
        instr.encode(&mut code);
        Instr::Halt.encode(&mut code);
        let p = program(&code);
        // Le préfixe dépense l'unique unité du budget : même les coûts 1
        // sont éprouvés avec zéro gaz restant, sans accepter un budget nul.
        let mut prefixed = vec![0x43];
        prefixed.extend_from_slice(&code);
        if cost > 0 {
            let result = machine
                .run::<Infallible>(&program(&prefixed), budget(cost), |_, _, _| {
                    panic!("instruction refusée exécutée : {instr:?}")
                })
                .unwrap();
            assert_eq!((result.fault, result.gas_used), (Some(Fault::OutOfGas), 1));
            assert_eq!(machine.pc(), 1);
            assert!(machine.state().stack.as_slice().is_empty());
        }
        for available in [cost.max(1), cost + 1] {
            let mut called = false;
            let result = machine
                .run::<Infallible>(&p, budget(available), |_, actual, pc| {
                    assert_eq!((actual, pc), (instr, 0));
                    called = true;
                    // Une faute injectée isole le cycle de la sémantique métier.
                    Err(Fault::BadJump.into())
                })
                .unwrap();
            let expected = match instr {
                Instr::Halt | Instr::JumpDest | Instr::Gas => None,
                Instr::Trace => Some(Fault::StackUnderflow),
                _ => Some(Fault::BadJump),
            };
            assert_eq!(
                (result.fault, result.gas_used),
                (expected, cost),
                "{instr:?}"
            );
            assert_eq!(called, expected == Some(Fault::BadJump));
            assert_eq!(machine.gas_remaining(), available - cost);
        }
    }
}

#[test]
fn gas_reports_remaining_after_its_charge_at_budget_limits() {
    let mut machine = Machine::default();
    let p = program(&[0x71, 0x00]);
    for available in [1, 2, 3, 1000, u16::MAX] {
        let result = machine.run(&p, budget(available), execute).unwrap();
        if available == 1 {
            assert_eq!((result.fault, result.gas_used), (Some(Fault::OutOfGas), 0));
            assert!(machine.state().stack.as_slice().is_empty());
        } else {
            assert_eq!((result.fault, result.gas_used), (None, 2));
            assert_eq!(
                machine.state().stack.as_slice(),
                &[i64::from(available - 2)]
            );
        }
    }
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
