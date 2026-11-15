use super::*;
use std::convert::Infallible;

fn program(code: &[u8]) -> Program {
    Program::new(vec![], code.to_vec()).unwrap()
}
fn budget(value: u16) -> NonZeroU16 {
    NonZeroU16::new(value).unwrap()
}

// Instructions restantes nécessaires aux tests du cycle.
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
                Instr::Halt | Instr::JumpDest | Instr::Gas | Instr::PushI16(_) => None,
                Instr::Const(_) => Some(Fault::BadConst),
                _ if execute_core(&mut FixedStack::default(), &p, instr).is_some() => {
                    Some(Fault::StackUnderflow)
                }
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

fn core_run(values: &[i64], instr: Instr) -> (Machine, Option<Fault>) {
    let p = Program::new(values.to_vec(), {
        let mut code = Vec::new();
        for index in 0..values.len() {
            Instr::Const(index as u16).encode(&mut code);
        }
        instr.encode(&mut code);
        Instr::Halt.encode(&mut code);
        code
    })
    .unwrap();
    let mut machine = Machine::default();
    let fault = machine
        .run::<Infallible>(&p, budget(1000), |_, instr, _| {
            panic!("instruction déléguée : {instr:?}")
        })
        .unwrap()
        .fault;
    (machine, fault)
}

#[test]
fn core_stack_effects_and_signed_immediates() {
    for (instr, input, expected) in [
        (Instr::Drop, vec![1, 2], vec![1]),
        (Instr::Dup, vec![1, 2], vec![1, 2, 2]),
        (Instr::Swap, vec![1, 2], vec![2, 1]),
        (Instr::Over, vec![1, 2], vec![1, 2, 1]),
        (Instr::Rot, vec![1, 2, 3], vec![2, 3, 1]),
        (Instr::PushI16(i16::MIN), vec![], vec![-32768]),
        (Instr::ConstX, vec![42, 0], vec![42, 42]),
    ] {
        let (m, f) = core_run(&input, instr);
        assert_eq!(f, None);
        assert_eq!(m.state().stack.as_slice(), expected);
    }
}

#[test]
fn arithmetic_logic_and_operand_order() {
    for (instr, input, expected) in [
        (Instr::Add, vec![i64::MAX, 1], i64::MIN),
        (Instr::Sub, vec![i64::MIN, 1], i64::MAX),
        (Instr::Sub, vec![7, 2], 5),
        (Instr::Mul, vec![i64::MAX, 2], -2),
        (Instr::Div, vec![i64::MIN, -1], i64::MIN),
        (Instr::Mod, vec![i64::MIN, -1], 0),
        (Instr::Div, vec![-7, 2], -3),
        (Instr::Mod, vec![-7, 2], -1),
        (Instr::Neg, vec![i64::MIN], i64::MIN),
        (Instr::Eq, vec![7, 7], 1),
        (Instr::Eq, vec![7, 2], 0),
        (Instr::Lt, vec![-1, 2], 1),
        (Instr::Lt, vec![2, -1], 0),
        (Instr::Gt, vec![2, -1], 1),
        (Instr::Gt, vec![-1, 2], 0),
        (Instr::Not, vec![0], 1),
        (Instr::Not, vec![-1], 0),
        (Instr::BNot, vec![0], -1),
        (Instr::BNot, vec![-1], 0),
        (Instr::And, vec![-1, 10], 10),
        (Instr::Or, vec![8, 3], 11),
        (Instr::Xor, vec![-1, 10], -11),
    ] {
        let (m, f) = core_run(&input, instr);
        assert_eq!(f, None, "{instr:?}");
        assert_eq!(m.state().stack.as_slice(), &[expected], "{instr:?}");
    }
}

#[test]
fn shifts_cover_sign_and_count_boundaries() {
    for a in [i64::MIN, -7, -1, 0, 7, i64::MAX] {
        for n in [i64::MIN, -1, 0, 1, 63, 64, 65, i64::MAX] {
            for instr in [Instr::Shl, Instr::Shr, Instr::Sar] {
                let expected = match instr {
                    Instr::Shl => {
                        if (0..64).contains(&n) {
                            ((a as u64) << n) as i64
                        } else {
                            0
                        }
                    }
                    Instr::Shr => {
                        if (0..64).contains(&n) {
                            ((a as u64) >> n) as i64
                        } else {
                            0
                        }
                    }
                    _ => {
                        if (0..64).contains(&n) {
                            a >> n
                        } else {
                            i64::from(a < 0).wrapping_neg()
                        }
                    }
                };
                let (m, f) = core_run(&[a, n], instr);
                assert_eq!(f, None);
                assert_eq!(m.state().stack.as_slice(), &[expected], "{a} {n} {instr:?}");
            }
        }
    }
}

#[test]
fn core_faults_and_stack_limits() {
    let operations = [
        (Instr::Drop, 1),
        (Instr::Dup, 1),
        (Instr::Swap, 2),
        (Instr::Over, 2),
        (Instr::Rot, 3),
        (Instr::ConstX, 1),
        (Instr::Neg, 1),
        (Instr::Not, 1),
        (Instr::BNot, 1),
        (Instr::Add, 2),
        (Instr::Sub, 2),
        (Instr::Mul, 2),
        (Instr::Div, 2),
        (Instr::Mod, 2),
        (Instr::Eq, 2),
        (Instr::Lt, 2),
        (Instr::Gt, 2),
        (Instr::And, 2),
        (Instr::Or, 2),
        (Instr::Xor, 2),
        (Instr::Shl, 2),
        (Instr::Shr, 2),
        (Instr::Sar, 2),
    ];
    for (instr, required) in operations {
        for count in 0..required {
            assert_eq!(
                core_run(&vec![1; count], instr).1,
                Some(Fault::StackUnderflow),
                "{instr:?}"
            );
        }
    }
    for instr in [Instr::Div, Instr::Mod] {
        assert_eq!(core_run(&[1, 0], instr).1, Some(Fault::DivByZero));
    }
    for index in [-1, 1, i64::MAX] {
        assert_eq!(core_run(&[index], Instr::ConstX).1, Some(Fault::BadConst));
    }
    assert_eq!(core_run(&[], Instr::Const(0)).1, Some(Fault::BadConst));
    for instr in [Instr::PushI16(1), Instr::Const(0), Instr::Dup, Instr::Over] {
        assert_eq!(core_run(&[1; 64], instr).1, Some(Fault::StackOverflow));
        assert_eq!(core_run(&[1; 63], instr).1, None);
    }
    for instr in [Instr::Swap, Instr::Rot, Instr::ConstX, Instr::Add] {
        assert_eq!(core_run(&[1; 64], instr).1, None);
    }
}
