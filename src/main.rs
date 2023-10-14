use crate::vm::BlockTarget;

mod jit;
mod vm;

fn main() {
    let mut vm = vm::VM::new(8, 1);
    let program_iters = 100_000_000;

    match std::env::args().skip(1).next().as_deref() {
        Some("--no-jit") => {
            let program = sample_loop_program(program_iters);
            program.dump();

            run_interpreted(&program, &mut vm).expect("failed to run program");
            vm.dump();
        }
        Some("--nop") => {
            let jit = jit::Jit::dummy();
            jit.dump();

            let executable = jit.into_exec();
            executable.run(&mut vm);
        }
        None => {
            let program = sample_loop_program(program_iters);
            program.dump();

            let jit = jit::Jit::compile(&program);
            jit.dump();

            let executable = jit.into_exec();
            executable.run(&mut vm);
            vm.dump();
        }
        Some(_) => {
            eprintln!("Usage: cheekyjit [--no-jit|--nop]");
            std::process::exit(1);
        }
    }
}

fn run_interpreted(program: &vm::Program, vm: &mut vm::VM) -> Result<(), ()> {
    let current_block = program.blocks.first().ok_or(())?.clone();
    let mut current_block = BlockTarget::new(current_block);
    let mut instruction_index = 0;

    while instruction_index < current_block.len() {
        let instruction = &current_block.instruction(instruction_index);
        match &instruction {
            vm::Instruction::LoadImmediate { value } => *vm.accum_reg_mut() = *value,
            vm::Instruction::Load { reg } => *vm.accum_reg_mut() = get_reg(vm, reg)?,
            vm::Instruction::Store { reg } => *get_reg_mut(vm, reg)? = *vm.accum_reg(),
            vm::Instruction::SetLocal { local } => *get_local_mut(vm, local)? = *vm.accum_reg(),
            vm::Instruction::GetLocal { local } => *vm.accum_reg_mut() = get_local(vm, local)?,
            vm::Instruction::Increment => vm.accum_reg_mut().0 += 1,
            vm::Instruction::LessThan { lhs } => vm.accum_reg_mut().0 = less_than(vm, lhs)?,
            vm::Instruction::Exit => return Ok(()),
            vm::Instruction::Jump { target } => {
                jump(&mut current_block, &mut instruction_index, target)
            }
            vm::Instruction::JumpConditional {
                true_target: t,
                false_target: f,
            } => {
                let target = if vm.accum_reg().0 != 0 { t } else { f };
                jump(&mut current_block, &mut instruction_index, target)
            }
        }
        instruction_index += 1;
    }

    fn get_reg(vm: &vm::VM, reg: &vm::VMRegister) -> Result<vm::Value, ()> {
        vm.registers.get(reg.0).ok_or(()).copied()
    }

    fn get_reg_mut<'a>(vm: &'a mut vm::VM, reg: &vm::VMRegister) -> Result<&'a mut vm::Value, ()> {
        vm.registers.get_mut(reg.0).ok_or(())
    }

    fn get_local(vm: &vm::VM, local: &vm::VMLocal) -> Result<vm::Value, ()> {
        vm.locals.get(local.0).ok_or(()).copied()
    }

    fn get_local_mut<'a>(vm: &'a mut vm::VM, local: &vm::VMLocal) -> Result<&'a mut vm::Value, ()> {
        vm.locals.get_mut(local.0).ok_or(())
    }

    fn less_than(vm: &vm::VM, lhs: &vm::VMRegister) -> Result<u64, ()> {
        let is_lt = get_reg(vm, lhs)?.0 < vm.accum_reg().0;
        Ok(if is_lt { 1 } else { 0 })
    }

    fn jump(dst: &mut BlockTarget, instruction_index: &mut usize, target: &vm::BlockTarget) {
        *dst = target.clone();
        *instruction_index = 0;
    }

    Ok(())
}

fn sample_loop_program(iters: u64) -> vm::Program {
    let mut program = vm::Program::default();
    let block1 = program.make_block();
    let block2 = program.make_block();
    let block3 = program.make_block();
    let block4 = program.make_block();
    let block5 = program.make_block();
    let block6 = program.make_block();

    block1.append(vm::Instruction::Store {
        reg: vm::VMRegister(5),
    });
    block1.append(vm::Instruction::LoadImmediate {
        value: vm::Value(0),
    });
    block1.append(vm::Instruction::SetLocal {
        local: vm::VMLocal(0),
    });
    block1.append(vm::Instruction::Load {
        reg: vm::VMRegister(5),
    });
    block1.append(vm::Instruction::LoadImmediate {
        value: vm::Value(0),
    });
    block1.append(vm::Instruction::Store {
        reg: vm::VMRegister(6),
    });
    block1.append(vm::Instruction::Jump {
        target: block4.clone(),
    });

    block2.append(vm::Instruction::Exit);

    block3.append(vm::Instruction::LoadImmediate {
        value: vm::Value(0),
    });
    block3.append(vm::Instruction::Jump {
        target: block5.clone(),
    });

    block4.append(vm::Instruction::GetLocal {
        local: vm::VMLocal(0),
    });
    block4.append(vm::Instruction::Store {
        reg: vm::VMRegister(7),
    });
    block4.append(vm::Instruction::LoadImmediate {
        value: vm::Value(iters),
    });
    block4.append(vm::Instruction::LessThan {
        lhs: vm::VMRegister(7),
    });
    block4.append(vm::Instruction::JumpConditional {
        true_target: block3.clone(),
        false_target: block6.clone(),
    });

    block5.append(vm::Instruction::Store {
        reg: vm::VMRegister(6),
    });
    block5.append(vm::Instruction::GetLocal {
        local: vm::VMLocal(0),
    });
    block5.append(vm::Instruction::Increment);
    block5.append(vm::Instruction::SetLocal {
        local: vm::VMLocal(0),
    });
    block5.append(vm::Instruction::Jump {
        target: block4.clone(),
    });

    block6.append(vm::Instruction::Load {
        reg: vm::VMRegister(6),
    });
    block6.append(vm::Instruction::Jump {
        target: block2.clone(),
    });
    program
}

pub fn env_var_flag_is_set(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .filter(|x| matches!(x.trim().parse(), Ok(1_usize)))
        .is_some()
}
