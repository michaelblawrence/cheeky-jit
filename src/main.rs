use std::fmt::Display;

use parser::Parser;
use vm::BlockTarget;

mod jit;
mod parser;
mod vm;

fn main() {
    let mut vm = vm::VM::new(8, 4);
    let program_iters = 100_000_000;

    match std::env::args().skip(1).next().as_deref() {
        Some("--no-jit") => {
            let program = sample_loop_program(program_iters);
            program.dump();

            run_interpreted(&program, &mut vm)
                .unwrap_or_else(|_| exit_with_error_msg("Failed to run program", ""));

            vm.dump();
            assert_eq!(
                vm.locals[0].0, program_iters,
                "program should set local[0] to 0"
            );
        }
        Some("--nop") => {
            let jit = jit::Jit::dummy();
            jit.dump();

            let executable = jit.into_exec();
            executable.run(&mut vm);
        }
        Some("-i") => {
            let path = std::env::args()
                .skip(2)
                .next()
                .unwrap_or_else(|| exit_with_usage_help());

            let code = std::fs::read_to_string(&path).unwrap_or_else(|err| {
                exit_with_error_msg(&format!("Failed to read provided file: {path}"), err)
            });
            let program = Parser::new(&code).parse().unwrap_or_else(|err| {
                exit_with_error_msg(&format!("Failed to compile program: {path}"), err)
            });

            program.dump();

            let jit = jit::Jit::compile(&program);
            jit.dump();

            let executable = jit.into_exec();
            executable.run(&mut vm);
            vm.dump();
        }
        None => {
            let program = sample_loop_program(program_iters);
            program.dump();

            let jit = jit::Jit::compile(&program);
            jit.dump();

            let executable = jit.into_exec();
            executable.run(&mut vm);
            vm.dump();

            assert_eq!(
                vm.locals[0].0, program_iters,
                "program should set local[0] to 0"
            );
        }
        Some(_) => {
            exit_with_usage_help();
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
    let sample_looper_code = format!(
        r#"
ENTRY:
  LOAD_IMM 0
  STORE_REG r1
  JUMP #LOOP0
LOOP0:
  LOAD_IMM {iters}
  LESS_THAN r1
  JUMP_EITHER #LOOP0_BODY #LOOP0_END
LOOP0_BODY:
  LOAD_REG r1
  INCR
  STORE_REG r1
  JUMP #LOOP0
LOOP0_END:
  LOAD_REG r1
  SET_LOCAL .0
  RET
"#
    );

    Parser::new(&sample_looper_code)
        .parse()
        .expect("failed to parse sample program;")
}

pub fn env_var_flag_is_set(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .filter(|x| matches!(x.trim().parse(), Ok(1_usize)))
        .is_some()
}

fn exit_with_usage_help() -> ! {
    eprintln!("Usage: cheekyjit [--no-jit|--nop|-i <bytecode_fpath>]");
    std::process::exit(1)
}

fn exit_with_error_msg(msg: &str, err: impl Display) -> ! {
    eprintln!("ERROR: {msg}");
    eprintln!("    {err}");
    std::process::exit(1)
}
