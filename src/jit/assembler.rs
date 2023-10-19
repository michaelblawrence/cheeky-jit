use crate::{vm::BlockTarget, vm::VMLocal, vm::VMRegister};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    GPR0 = 4, // x4
    GPR1 = 5, // x5

    VmStructBase = 0,     // x0
    RegisterArrayBase = 1, // x1
    LocalsArrayBase = 2,   // x2

    RET = 30,
    SP = 31,
}

pub enum Func {
    FnSingleInt64WithReturnInt64(fn(u64) -> u64, u64),
}

#[derive(Debug, Clone, Copy)]
enum Operand {
    Reg(Reg),
    Imm64(u64),
    Mem64BaseAndOffset(Reg, usize),
}

#[derive(Default)]
pub struct Assembler {
    output: Vec<u8>,
}

impl Assembler {
    pub fn load_immediate64(&mut self, dst: Reg, imm: u64) {
        self.mov(Operand::Reg(dst), Operand::Imm64(imm));
    }

    pub fn store_vm_register(&mut self, dst: VMRegister, src: Reg) {
        self.mov(
            Operand::Mem64BaseAndOffset(Reg::RegisterArrayBase, dst.0),
            Operand::Reg(src),
        );
    }

    pub fn load_vm_register(&mut self, dst: Reg, src: VMRegister) {
        self.mov(
            Operand::Reg(dst),
            Operand::Mem64BaseAndOffset(Reg::RegisterArrayBase, src.0),
        );
    }

    pub fn store_vm_local(&mut self, dst: VMLocal, src: Reg) {
        self.mov(
            Operand::Mem64BaseAndOffset(Reg::LocalsArrayBase, dst.0),
            Operand::Reg(src),
        );
    }

    pub fn load_vm_local(&mut self, dst: Reg, src: VMLocal) {
        self.mov(
            Operand::Reg(dst),
            Operand::Mem64BaseAndOffset(Reg::LocalsArrayBase, src.0),
        );
    }

    pub fn increment(&mut self, dst: Reg) {
        // // Add 1 to the value in dst register
        self.writer().emit_incr(dst);
    }

    pub fn less_than(&mut self, dst: Reg, src: Reg) {
        // // Compare src and dst registers
        self.writer().emit_cmp(src, Operand::Reg(dst));

        // Set dst to 1 if src < dst, else set it to 0
        self.writer().emit_cset(dst);
    }

    pub fn jump(&mut self, target: &BlockTarget) {
        // Branch to the target basic block (26-bit offset)
        self.writer().emit_branch(0xdeadaf);
        target.insert_jump_marker(self.len());
    }

    pub fn jump_conditional(
        &mut self,
        reg: Reg,
        true_target: &BlockTarget,
        false_target: &BlockTarget,
    ) {
        // Compare reg with zero
        self.writer().emit_cmp(reg, Operand::Imm64(0));

        // Branch to false_target if reg is zero
        self.writer().emit_branch_eq(0xdead); // Replace with the correct branch offset
        false_target.insert_jump_marker(self.len());

        // Branch to true_target (unconditionally)
        self.jump(true_target);
    }

    pub fn call_into_rust(&mut self, dst: Reg, func: Func) {
        match func {
            Func::FnSingleInt64WithReturnInt64(func, arg0) => {
                let addr = func as *const () as u64;
                self.writer().emit_push(Reg::VmStructBase);
                self.writer().emit_push(Reg::RegisterArrayBase);
                self.writer().emit_push(Reg::LocalsArrayBase);
                self.writer().emit_push(Reg::GPR0);
                self.writer().emit_push(Reg::GPR1);
                self.writer().emit_push(Reg::RET);

                self.writer().emit_mov_imm(Reg::VmStructBase, arg0);
                self.writer().emit_mov_imm(Reg::GPR1, addr);
                self.writer().emit_branch_with_link(Reg::GPR1);
                self.writer().emit_mov_reg(Reg::GPR0, Reg::VmStructBase);

                self.writer().emit_pop(Some(Reg::RET));
                self.writer().emit_pop(Some(Reg::GPR1));
                if dst != Reg::GPR0 {
                    self.writer().emit_mov_reg(dst, Reg::GPR0);
                    self.writer().emit_pop(Some(Reg::GPR0));
                } else {
                    self.writer().emit_pop(None);
                }
                self.writer().emit_pop(Some(Reg::LocalsArrayBase));
                self.writer().emit_pop(Some(Reg::RegisterArrayBase));
                self.writer().emit_pop(Some(Reg::VmStructBase));
            }
        }
    }

    pub fn brk(&mut self) {
        self.writer().emit_brk(0);
    }

    pub fn ret(&mut self) {
        // Return from the function
        self.writer().emit_ret();
    }

    pub fn no_op(&mut self) {
        self.writer().emit_nop();
    }

    pub fn rewrite_instr32(&mut self, offset: usize, value: u32) {
        for i in 0..4 {
            self.output[offset + i] = ((value >> (i * 8)) & 0xff) as u8;
        }
    }

    fn mov(&mut self, dst: Operand, src: Operand) {
        match (dst, src) {
            (Operand::Reg(dst), Operand::Reg(src)) => {
                // Move from src register to dst register
                self.writer().emit_mov_reg(dst, src);
            }
            (Operand::Reg(dst), Operand::Imm64(imm)) => {
                self.writer().emit_mov_imm(dst, imm);
            }
            (Operand::Mem64BaseAndOffset(dst, dst_offset), Operand::Reg(src)) => {
                // Store from src register to memory location pointed to by dst
                self.writer().emit_str(dst, dst_offset, src);
            }
            (Operand::Reg(dst), Operand::Mem64BaseAndOffset(src, src_offset)) => {
                // Load from memory location pointed to by src to dst register
                assert_eq!(src_offset >> 12, 0);
                self.writer().emit_ldr(dst, src, src_offset);
            }
            _ => panic!("unrecognized mov instruction"),
        }
    }

    fn writer(&mut self) -> Arm64Writer {
        Arm64Writer(&mut self.output)
    }
}

impl std::ops::Deref for Assembler {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.output
    }
}

impl std::ops::DerefMut for Assembler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.output
    }
}

pub struct BitIndex {
    pub bits: usize,
    pub value: usize,
}

struct Arm64Writer<'a>(&'a mut Vec<u8>);

impl<'a> Arm64Writer<'a> {
    pub fn emit_mov_reg(&mut self, dst: Reg, src: Reg) {
        // 10101010000 Rm00000011111
        // MOV (register)
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b10101010000,
                bits: 11,
            }),
            1 => Some(BitIndex {
                value: src as usize,
                bits: 5,
            }),
            2 => Some(BitIndex {
                value: 0b00000011111,
                bits: 11,
            }),
            3 => Some(BitIndex {
                value: dst as usize,
                bits: 5,
            }),
            _ => None,
        })
        .unwrap();
    }

    pub fn emit_mov_imm(&mut self, dst: Reg, imm: u64) {
        // Move immediate value to dst register
        // MOVZ
        const IMM16_MASK: usize = (1 << 16) - 1;
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b11010010100,
                bits: 11,
            }),
            1 => Some(BitIndex {
                value: (imm as usize) & IMM16_MASK,
                bits: 16,
            }),
            2 => Some(BitIndex {
                value: dst as usize,
                bits: 5,
            }),
            _ => None,
        })
        .unwrap();

        let mut imm = imm >> 16;
        let mut hw = 1;
        while imm != 0 && hw < 4 {
            self.emit32_gen(|idx| match idx {
                0 => Some(BitIndex {
                    value: 0b111100101,
                    bits: 9,
                }),
                1 => Some(BitIndex { value: hw, bits: 2 }),
                2 => Some(BitIndex {
                    value: (imm as usize) & IMM16_MASK,
                    bits: 16,
                }),
                3 => Some(BitIndex {
                    value: dst as usize,
                    bits: 5,
                }),
                _ => None,
            })
            .unwrap();

            hw += 1;
            imm = imm >> 16;
        }
    }

    pub fn emit_str(&mut self, dst: Reg, dst_offset: usize, src: Reg) {
        // Store register (STR)
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b1111100100,
                bits: 10,
            }),
            1 => Some(BitIndex {
                value: dst_offset,
                bits: 12,
            }),
            2 => Some(BitIndex {
                value: dst as usize,
                bits: 5,
            }),
            3 => Some(BitIndex {
                value: src as usize,
                bits: 5,
            }),
            _ => None,
        })
        .unwrap();
    }

    pub fn emit_ldr(&mut self, dst: Reg, src: Reg, src_offset: usize) {
        // LDR (immediate)
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b1111100101,
                bits: 10,
            }),
            1 => Some(BitIndex {
                value: src_offset,
                bits: 12,
            }),
            2 => Some(BitIndex {
                value: src as usize,
                bits: 5,
            }),
            3 => Some(BitIndex {
                value: dst as usize,
                bits: 5,
            }),
            _ => None,
        })
        .unwrap();
    }

    pub fn emit_cset(&mut self, dst: Reg) {
        // CSET <Xd>, <cond>
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b1001101010011111,
                bits: 16,
            }),
            1 => Some(BitIndex {
                value: 0b1101, // cond = LT (Signed less than)
                bits: 4,
            }),
            2 => Some(BitIndex {
                value: 0b0111111,
                bits: 7,
            }),
            3 => Some(BitIndex {
                value: dst as usize,
                bits: 5,
            }),
            _ => None,
        })
        .unwrap();
    }

    pub fn emit_add(&mut self, dst: Reg, src: Reg, value: u16) {
        // ADD (immediate)
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b1001000100,
                bits: 10,
            }),
            1 => Some(BitIndex {
                value: value as usize,
                bits: 12,
            }),
            2 => Some(BitIndex {
                value: src as usize,
                bits: 5,
            }),
            3 => Some(BitIndex {
                value: dst as usize,
                bits: 5,
            }),
            _ => None,
        })
        .unwrap();
    }

    pub fn emit_sub(&mut self, dst: Reg, src: Reg, value: u16) {
        // SUB (immediate)
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b1101000100,
                bits: 10,
            }),
            1 => Some(BitIndex {
                value: value as usize,
                bits: 12,
            }),
            2 => Some(BitIndex {
                value: src as usize,
                bits: 5,
            }),
            3 => Some(BitIndex {
                value: dst as usize,
                bits: 5,
            }),
            _ => None,
        })
        .unwrap();
    }

    pub fn emit_push(&mut self, src: Reg) {
        self.emit_sub(Reg::SP, Reg::SP, 64); // 64-bit
        self.emit_str(Reg::SP, 1, src);
    }

    pub fn emit_pop(&mut self, dst: Option<Reg>) {
        if let Some(dst) = dst {
            self.emit_ldr(dst, Reg::SP, 1);
        }
        self.emit_add(Reg::SP, Reg::SP, 64); // 64-bit
    }

    pub fn emit_branch(&mut self, addr_offset: usize) {
        // B (Branch)
        // Branch to target (26-bit offset)
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b000101,
                bits: 6,
            }),
            1 => Some(BitIndex {
                value: addr_offset,
                bits: 26,
            }),
            _ => None,
        })
        .unwrap();
    }

    pub fn emit_branch_with_link(&mut self, target: Reg) {
        // BLR (Branch with Link to Register)
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b1101011000111111000000,
                bits: 22,
            }),
            1 => Some(BitIndex {
                value: target as usize,
                bits: 5,
            }),
            2 => Some(BitIndex { value: 0, bits: 5 }),
            _ => None,
        })
        .unwrap();
    }

    pub fn emit_branch_eq(&mut self, imm19: usize) {
        // B.cond (cond = EQ)
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b01010100,
                bits: 8,
            }),
            1 => Some(BitIndex {
                value: imm19,
                bits: 19,
            }),
            2 => Some(BitIndex { value: 0, bits: 5 }),
            _ => None,
        })
        .unwrap();
    }

    pub fn emit_cmp(&mut self, lhs: Reg, rhs: Operand) {
        // lhs => n, rhs => m
        match rhs {
            Operand::Reg(rhs) => {
                // CMP (shifted register)
                // CMP <Xn>, <Xm>{, <shift> #<amount>}
                self.emit32_gen(|idx| match idx {
                    0 => Some(BitIndex {
                        value: 0b11101011000,
                        bits: 11,
                    }),
                    1 => Some(BitIndex {
                        value: rhs as usize,
                        bits: 5,
                    }),
                    2 => Some(BitIndex { value: 0, bits: 6 }),
                    3 => Some(BitIndex {
                        value: lhs as usize,
                        bits: 5,
                    }),
                    4 => Some(BitIndex {
                        value: 0b11111,
                        bits: 5,
                    }),
                    _ => None,
                })
                .unwrap();
            }
            Operand::Imm64(imm12) => {
                // CMP (immediate)
                // CMP <Xn|SP>, #<imm>{, <shift>}
                self.emit32_gen(|idx| match idx {
                    0 => Some(BitIndex {
                        value: 0b1111000100,
                        bits: 10,
                    }),
                    1 => Some(BitIndex {
                        value: imm12 as usize,
                        bits: 12,
                    }),
                    2 => Some(BitIndex {
                        value: lhs as usize,
                        bits: 5,
                    }),
                    3 => Some(BitIndex {
                        value: 0b11111,
                        bits: 5,
                    }),
                    _ => None,
                })
                .unwrap();
            }
            Operand::Mem64BaseAndOffset(_, _) => todo!("not supported"),
        }
    }

    pub fn emit_incr(&mut self, dst: Reg) {
        // add x1, x1, #1
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b1001000100,
                bits: 10,
            }),
            1 => Some(BitIndex {
                value: 1, // Immediate Value
                bits: 12,
            }),
            2 => Some(BitIndex {
                value: dst as usize,
                bits: 5,
            }),
            3 => Some(BitIndex {
                value: dst as usize,
                bits: 5,
            }),
            _ => None,
        })
        .unwrap();
    }

    pub fn emit_ret(&mut self) {
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b1101011001011111000000,
                bits: 22,
            }),
            1 => Some(BitIndex {
                value: 0b11110, // x30
                bits: 5,
            }),
            2 => Some(BitIndex { value: 0, bits: 5 }),
            _ => None,
        })
        .unwrap();
    }

    pub fn emit_brk(&mut self, imm16: u16) {
        // BRK
        self.emit32_gen(|idx| match idx {
            0 => Some(BitIndex {
                value: 0b11010100001,
                bits: 11,
            }),
            1 => Some(BitIndex {
                value: imm16 as usize,
                bits: 16,
            }),
            2 => Some(BitIndex { value: 0, bits: 5 }),
            _ => None,
        })
        .unwrap();
    }

    pub fn emit_nop(&mut self) {
        // NOP
        self.emit32(0b1101_0101_0000_0011_0010_0000_0001_1111);
    }

    fn emit32(&mut self, value: u32) {
        for i in 0..4 {
            self.0.push(((value >> (i * 8)) & 0xff) as u8);
        }
    }

    fn emit32_gen(&mut self, generator: impl FnMut(usize) -> Option<BitIndex>) -> Result<(), ()> {
        let value = BitwiseWriter::write(generator)?;
        self.emit32(value);
        Ok(())
    }
}
pub struct BitwiseWriter;

impl BitwiseWriter {
    pub fn write(mut generator: impl FnMut(usize) -> Option<BitIndex>) -> Result<u32, ()> {
        let mut bit_position = 0;
        let mut index = 0;
        let mut value: u32 = 0;
        let mut more_bits = bit_position < 32;

        while more_bits {
            more_bits = bit_position < 32;
            match generator(index) {
                Some(bits) => {
                    let shift = bits.bits as u32;
                    let mask: u32 = (1 << shift) - 1;
                    if (bits.value >> shift) != 0 {
                        panic!(
                            "overflow bit length: value = {} (max value = {mask}, gen_idx = {index})",
                            bits.value
                        );
                    }
                    bit_position += shift;
                    value = (value << shift) + (bits.value as u32 & mask)
                }
                None if more_bits => return Err(()),
                None => break,
            }
            index += 1;
        }

        match bit_position {
            32 => Ok(value),
            _ => Err(()),
        }
    }
}
