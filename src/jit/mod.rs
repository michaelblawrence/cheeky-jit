use crate::{env_var_flag_is_set, vm::Instruction, vm::Program, vm::VMRegister};

use self::assembler::{Func, Reg};

mod assembler;
mod executable;

#[derive(Default)]
pub struct Jit {
    assembler: assembler::Assembler,
    block_offsets: Vec<usize>,
}

impl Jit {
    pub fn compile(program: &Program) -> Self {
        let mut jit = Jit::default();
        let assembler = &mut jit.assembler;

        for block in program.blocks.iter() {
            block.borrow_mut().offset = assembler.len();
            jit.block_offsets.push(assembler.len());

            for instruction in &block.borrow().instructions {
                let instruction = instruction.borrow().clone();

                match instruction {
                    Instruction::LoadImmediate { value } => {
                        assembler.load_immediate64(Reg::GPR0, value.0);
                        assembler.store_vm_register(VMRegister(0), Reg::GPR0);
                    }
                    Instruction::Load { reg } => {
                        assembler.load_vm_register(Reg::GPR0, reg);
                        assembler.store_vm_register(VMRegister(0), Reg::GPR0);
                    }
                    Instruction::Store { reg } => {
                        assembler.load_vm_register(Reg::GPR0, VMRegister(0));
                        assembler.store_vm_register(reg, Reg::GPR0);
                    }
                    Instruction::SetLocal { local } => {
                        assembler.load_vm_register(Reg::GPR0, VMRegister(0));
                        assembler.store_vm_local(local, Reg::GPR0);
                    }
                    Instruction::GetLocal { local } => {
                        assembler.load_vm_local(Reg::GPR0, local);
                        assembler.store_vm_register(VMRegister(0), Reg::GPR0);
                    }
                    Instruction::Increment => {
                        assembler.load_vm_register(Reg::GPR0, VMRegister(0));
                        assembler.increment(Reg::GPR0);
                        assembler.store_vm_register(VMRegister(0), Reg::GPR0);
                    }
                    Instruction::LessThan { lhs } => {
                        assembler.load_vm_register(Reg::GPR0, lhs);
                        assembler.load_vm_register(Reg::GPR1, VMRegister(0));

                        assembler.less_than(Reg::GPR0, Reg::GPR1);
                        assembler.store_vm_register(VMRegister(0), Reg::GPR0);
                    }
                    Instruction::LoadRandom { max } => {
                        assembler.call_into_rust(
                            Reg::GPR0,
                            Func::FnSingleInt64WithReturnInt64(
                                crate::vm::rand::ParkMiller::next,
                                max.0,
                            ),
                        );
                        assembler.store_vm_register(VMRegister(0), Reg::GPR0);
                    }
                    Instruction::Breakpoint => {
                        assembler.brk();
                    }
                    Instruction::Exit => {
                        assembler.ret();
                    }
                    Instruction::Jump { target } => {
                        assembler.jump(&target);
                    }
                    Instruction::JumpConditional {
                        true_target,
                        false_target,
                    } => {
                        assembler.load_vm_register(Reg::GPR0, VMRegister(0));
                        assembler.jump_conditional(Reg::GPR0, &true_target, &false_target);
                    }
                }
            }
        }

        for block in &program.blocks {
            let block_offset = block.borrow().offset;
            for jump in block.borrow().jumps_to_here.iter().copied() {
                jit.link_and_rewrite(block_offset, jump);
            }
        }
        jit
    }

    fn link_and_rewrite(&mut self, target_offset: usize, instr_offset: usize) {
        const OP_JMP: u8 = 0b000101;
        const OP_JEQ: u8 = 0b010101;

        let jump_instr = &self.assembler[instr_offset..instr_offset + 4];
        let op_code = jump_instr[3] >> 2;

        let byte_offset = target_offset as i16 - instr_offset as i16;
        let offset = byte_offset / 4;

        let value = match op_code {
            OP_JMP => assembler::BitwiseWriter::write(|idx| match idx {
                0 => Some(assembler::BitIndex {
                    value: op_code as usize,
                    bits: 6,
                }),
                1 => Some(assembler::BitIndex {
                    value: sign_extend_upper_bits(offset, 10),
                    bits: 10,
                }),
                2 => Some(assembler::BitIndex {
                    value: sign_extend(offset, 16),
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
                    value: sign_extend_upper_bits(offset, 3),
                    bits: 3,
                }),
                2 => Some(assembler::BitIndex {
                    value: sign_extend(offset, 16),
                    bits: 16,
                }),
                3 => Some(assembler::BitIndex { value: 0, bits: 5 }),
                _ => None,
            }),
            b => todo!("handle additional jump instructions 0b{b:06x}"),
        };

        self.assembler.rewrite_instr32(instr_offset, value.unwrap());
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

    pub fn dump_exec_addr(&self, exec_start: *const u8) {
        eprintln!("block addresses: ");
        for (i, block_offset) in self.block_offsets.iter().enumerate() {
            let addr = exec_start as usize + block_offset;
            eprintln!("Block #{index} => 0x{addr:016x?}", index = i + 1);
        }
        eprintln!("");
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
}

fn sign_extend_upper_bits(value: i16, bits: usize) -> usize {
    if value.is_negative() {
        (1 << bits) - 1
    } else {
        0
    }
}

fn sign_extend(value: i16, bits: usize) -> usize {
    if value.is_negative() {
        let max_bit_value: i64 = 1 << bits;
        (max_bit_value + value as i64) as usize
    } else {
        value as usize
    }
}
