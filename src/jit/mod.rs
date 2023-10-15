use crate::{
    env_var_flag_is_set, vm::BlockTarget, vm::Instruction, vm::Program, vm::VMLocal,
    vm::VMRegister, vm::Value,
};

use self::assembler::Reg;

mod assembler;
mod executable;

#[derive(Default)]
pub struct Jit {
    assembler: assembler::Assembler,
}

impl Jit {
    pub fn compile(program: &Program) -> Self {
        let mut jit = Jit::default();
        for block in program.blocks.iter() {
            block.borrow_mut().offset = jit.assembler.len();

            for instruction in &block.borrow().instructions {
                let instruction = instruction.borrow().clone();
                match instruction {
                    Instruction::LoadImmediate { value } => jit.compile_load_immediate(value),
                    Instruction::Load { reg } => jit.compile_load(reg),
                    Instruction::Store { reg } => jit.compile_store(reg),
                    Instruction::SetLocal { local } => jit.compile_set_local(local),
                    Instruction::GetLocal { local } => jit.compile_get_local(local),
                    Instruction::Increment => jit.compile_increment(),
                    Instruction::LessThan { lhs } => jit.compile_less_than(lhs),
                    Instruction::Breakpoint => jit.compile_breakpoint(),
                    Instruction::Exit => jit.compile_exit(),
                    Instruction::Jump { target } => jit.compile_jump(&target),
                    Instruction::JumpConditional {
                        true_target,
                        false_target,
                    } => jit.compile_jump_conditional(&true_target, &false_target),
                }
            }
        }

        for block in &program.blocks {
            let block_offset = block.borrow().offset;
            for jump in block.borrow().jumps_to_here.iter().copied() {
                let byte_offset = block_offset as i16 - jump as i16;
                let offset = byte_offset / 4;
                let jump_instr = &jit.assembler[jump + 0..jump + 4];
                let op_code = jump_instr[3] >> 2;

                const OP_JMP: u8 = 0b000101;
                const OP_JEQ: u8 = 0b010101;

                let value = match op_code {
                    OP_JMP => assembler::BitwiseWriter::write(|idx| match idx {
                        0 => Some(assembler::BitIndex {
                            value: op_code as usize,
                            bits: 6,
                        }),
                        1 => Some(assembler::BitIndex {
                            value: sign_extend(offset, 10, true),
                            bits: 10,
                        }),
                        2 => Some(assembler::BitIndex {
                            value: sign_extend(offset, 16, false),
                            bits: 16,
                        }),
                        _ => None,
                    }),
                    OP_JEQ => assembler::BitwiseWriter::write(|idx| match idx {
                        0 => Some(assembler::BitIndex {
                            value: 0b01010100,
                            bits: 8,
                        }),
                        1 => Some(assembler::BitIndex {
                            value: sign_extend(offset, 3, true),
                            bits: 3,
                        }),
                        2 => Some(assembler::BitIndex {
                            value: sign_extend(offset, 16, false),
                            bits: 16,
                        }),
                        3 => Some(assembler::BitIndex { value: 0, bits: 5 }),
                        _ => None,
                    }),
                    b => todo!("handle additional jump instructions 0b{b:06x}"),
                };

                jit.assembler.rewrite_instr32(jump, value.unwrap());
            }
        }

        jit
    }

    pub fn dump(&self) {
        let len = self.assembler.len();
        let init = String::with_capacity(len * 4);

        let hex = self
            .assembler
            .chunks(2)
            .enumerate()
            .fold(init, |mut s, (i, x)| {
                use std::fmt::Write;
                if i % 8 == 0 {
                    s.push_str("    ");
                }
                for x in x {
                    write!(&mut s, "{:02x?}", *x).unwrap();
                }
                if i % 8 == 7 {
                    s.push('\n');
                } else {
                    s.push(' ');
                }
                s
            })
            .to_string();

        eprintln!("exec dump: ");
        eprintln!("{hex}");
        eprintln!("");

        self.bytecode_to_file();
    }

    pub fn into_exec(self) -> executable::Executable {
        if env_var_flag_is_set("DRY_RUN") {
            eprintln!("Dry run mode is enabled, quitting..");
            std::process::exit(0);
        }

        executable::Executable::new(self)
    }

    pub fn dummy() -> Self {
        let mut jit = Self::default();
        jit.assembler.no_op();
        jit.assembler.no_op();
        jit.assembler.no_op();
        jit.assembler.ret();
        jit
    }

    /// Safety: must ensure the dst buffer is at least as large as self.assembler.len()
    pub unsafe fn copy_into(&self, dst: *mut u8) {
        std::ptr::copy(self.assembler.as_ptr(), dst, self.assembler.len())
    }

    fn bytecode_to_file(&self) {
        use std::io::{BufWriter, Write};

        let file = std::fs::File::create("bytecode.out").unwrap();
        let mut writer = BufWriter::new(file);
        writer.write_all(&self.assembler[..]).unwrap();
        writer.flush().unwrap();
    }

    fn compile_load_immediate(&mut self, value: Value) {
        self.assembler.load_immediate64(Reg::GPR0, value.0);
        self.assembler.store_vm_register(VMRegister(0), Reg::GPR0);
    }

    fn compile_load(&mut self, reg: VMRegister) {
        self.assembler.load_vm_register(Reg::GPR0, reg);
        self.assembler.store_vm_register(VMRegister(0), Reg::GPR0);
    }

    fn compile_store(&mut self, reg: VMRegister) {
        self.assembler.load_vm_register(Reg::GPR0, VMRegister(0));
        self.assembler.store_vm_register(reg, Reg::GPR0);
    }

    fn compile_set_local(&mut self, local: VMLocal) {
        self.assembler.load_vm_register(Reg::GPR0, VMRegister(0));
        self.assembler.store_vm_local(local, Reg::GPR0);
    }

    fn compile_get_local(&mut self, local: VMLocal) {
        self.assembler.load_vm_local(Reg::GPR0, local);
        self.assembler.store_vm_register(VMRegister(0), Reg::GPR0);
    }

    fn compile_increment(&mut self) {
        self.assembler.load_vm_register(Reg::GPR0, VMRegister(0));
        self.assembler.increment(Reg::GPR0);
        self.assembler.store_vm_register(VMRegister(0), Reg::GPR0);
    }

    fn compile_less_than(&mut self, lhs: VMRegister) {
        self.assembler.load_vm_register(Reg::GPR0, lhs);
        self.assembler.load_vm_register(Reg::GPR1, VMRegister(0));

        self.assembler.less_than(Reg::GPR0, Reg::GPR1);

        self.assembler.store_vm_register(VMRegister(0), Reg::GPR0);
    }

    fn compile_jump(&mut self, target: &BlockTarget) {
        self.assembler.jump(target);
    }

    fn compile_jump_conditional(&mut self, true_target: &BlockTarget, false_target: &BlockTarget) {
        self.assembler.load_vm_register(Reg::GPR0, VMRegister(0));
        self.assembler
            .jump_conditional(Reg::GPR0, true_target, false_target);
    }

    fn compile_breakpoint(&mut self) {
        self.assembler.brk();
    }

    fn compile_exit(&mut self) {
        self.assembler.ret();
    }
}

fn sign_extend(value: i16, bits: usize, extended_bits: bool) -> usize {
    match (bits, extended_bits) {
        (16, false) => {
            if value.is_negative() {
                const I16_MAX: i64 = 1 << 16;
                (I16_MAX + value as i64) as usize
            } else {
                value as usize
            }
        }
        (_, true) => {
            if value.is_negative() {
                (1 << bits) - 1
            } else {
                0
            }
        }
        (_, false) => panic!("must read 16 non-extended bits"),
    }
}
