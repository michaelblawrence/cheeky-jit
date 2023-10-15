use std::collections::HashMap;

use crate::{
    parser::from_str::{VMLocalTarget, VMRegisterTarget},
    vm,
};

#[derive(Clone)]
enum ParserState {
    BlockStart,
    BlockInstructions(vm::BlockTarget),
}

pub struct Parser<'a> {
    code: &'a str,
    state: ParserState,
    program: vm::Program,
    blocks_with_declarations: Vec<String>,
    block_targets: HashMap<String, vm::BlockTarget>,
}

impl<'a> Parser<'a> {
    pub fn new(code: &'a str) -> Self {
        Self {
            code,
            state: ParserState::BlockStart,
            program: Default::default(),
            blocks_with_declarations: Default::default(),
            block_targets: Default::default(),
        }
    }

    pub fn parse(mut self) -> Result<vm::Program, String> {
        for (line_idx, line) in self.code.lines().enumerate() {
            let i = line_idx + 1; // line_num
            if !line.is_empty() && line.chars().next().unwrap().is_alphanumeric() {
                self.state = ParserState::BlockStart;
            }

            let line = line.trim();
            let line = line.split_once("//").map_or(line, |(line, _)| line.trim());
            if line.is_empty() {
                continue;
            }

            self.state = match self.state.clone() {
                ParserState::BlockStart if line.ends_with(':') => {
                    let block = self.parse_block_start(line);
                    ParserState::BlockInstructions(block)
                }
                ParserState::BlockStart => Err(format!("expected block label on line {i}"))?,
                ParserState::BlockInstructions(block) => {
                    self.parse_block_instructions(line, &block, i)?;
                    ParserState::BlockInstructions(block.clone())
                }
            }
        }

        self.validate_all_blocks_are_declared()?;
        Ok(self.program)
    }

    fn parse_block_start(&mut self, line: &str) -> vm::BlockTarget {
        let label = line
            .chars()
            .take_while(|x| x.is_alphanumeric() || *x == '_')
            .collect();

        assert!(!self.blocks_with_declarations.contains(&label));
        self.blocks_with_declarations.push(label.clone());
        self.get_or_create_block(label)
    }

    fn parse_block_instructions(
        &mut self,
        line: &str,
        b: &vm::BlockTarget,
        i: usize,
    ) -> Result<(), String> {
        Ok(match line.split_once(" ") {
            Some(("LOAD_IMM", x)) => instruction::add_single_operand(b, x, i, |x: u64| {
                Ok(vm::Instruction::LoadImmediate {
                    value: vm::Value(x),
                })
            })?,
            Some(("LOAD_REG", x)) => {
                instruction::add_single_operand(b, x, i, |x: VMRegisterTarget| {
                    Ok(vm::Instruction::Load { reg: x.0 })
                })?
            }
            Some(("STORE_REG", x)) => {
                instruction::add_single_operand(b, x, i, |x: VMRegisterTarget| {
                    Ok(vm::Instruction::Store { reg: x.0 })
                })?
            }
            Some(("SET_LOCAL", x)) => {
                instruction::add_single_operand(b, x, i, |x: VMLocalTarget| {
                    Ok(vm::Instruction::SetLocal { local: x.0 })
                })?
            }
            Some(("GET_LOCAL", x)) => {
                instruction::add_single_operand(b, x, i, |x: VMLocalTarget| {
                    Ok(vm::Instruction::GetLocal { local: x.0 })
                })?
            }
            Some(("LESS_THAN", x)) => {
                instruction::add_single_operand(b, x, i, |x: VMRegisterTarget| {
                    Ok(vm::Instruction::LessThan { lhs: x.0 })
                })?
            }
            Some(("JUMP", x)) => instruction::add_single_operand(b, x, i, |x: String| {
                Ok(vm::Instruction::Jump {
                    target: self.block_target_literal(&x)?,
                })
            })?,
            Some(("JUMP_EITHER", x)) => {
                instruction::add_double_operand(b, x, i, |t: String, f: String| {
                    Ok(vm::Instruction::JumpConditional {
                        true_target: self.block_target_literal(&t)?,
                        false_target: self.block_target_literal(&f)?,
                    })
                })?
            }
            None if line == "INCR" => instruction::add_unary(b, vm::Instruction::Increment),
            None if line == "BREAK" => instruction::add_unary(b, vm::Instruction::Breakpoint),
            None if line == "RET" => instruction::add_unary(b, vm::Instruction::Exit),

            Some((instr, _)) => Err(format!("unexpected instruction `{instr}` on line {i}"))?,
            None => Err(format!("unexpected unary instruction `{line}` on line {i}"))?,
        })
    }

    fn block_target_literal(&mut self, x: &str) -> Result<vm::BlockTarget, String> {
        let block_label = from_str::extract_prefix(x.trim(), '#');
        let block_label = block_label.map_err(|_| format!("unexpected block reference `{x}`"))?;
        Ok(self.get_or_create_block(block_label))
    }

    fn get_or_create_block(&mut self, block_label: String) -> vm::BlockTarget {
        let block = self
            .block_targets
            .entry(block_label)
            .or_insert_with(|| self.program.make_block())
            .clone();
        block
    }

    fn validate_all_blocks_are_declared(&self) -> Result<(), String> {
        let referenced_block_labels = self.block_targets.keys();

        let undeclared_blocks: Vec<_> = referenced_block_labels
            .filter(|referenced_label| !self.blocks_with_declarations.contains(referenced_label))
            .map(|x| x.as_str())
            .collect();

        if undeclared_blocks.is_empty() {
            Ok(())
        } else {
            let list = undeclared_blocks.join(", ");
            Err(format!(
                "missing declaration for the following block reference literal(s): {}",
                list
            ))
        }
    }
}

mod instruction {
    use std::{fmt::Display, str::FromStr};

    use crate::vm;

    pub fn add_unary(block: &vm::BlockTarget, instr: vm::Instruction) {
        block.append(instr);
    }

    pub fn add_single_operand<T>(
        block: &vm::BlockTarget,
        x: &str,
        line_num: usize,
        f: impl FnOnce(T) -> Result<vm::Instruction, String>,
    ) -> Result<(), String>
    where
        T: FromStr,
        T::Err: Display,
    {
        let x: T = x
            .parse()
            .map_err(|err| format!("failed to parse on line {}: {err}", line_num))?;

        let instruction =
            f(x).map_err(|err| format!("failed to parse on line {}: {err}", line_num))?;

        Ok(add_unary(block, instruction))
    }

    pub fn add_double_operand<T1, T2>(
        block: &vm::BlockTarget,
        x: &str,
        line_num: usize,
        f: impl FnOnce(T1, T2) -> Result<vm::Instruction, String>,
    ) -> Result<(), String>
    where
        T1: FromStr,
        T1::Err: Display,
        T2: FromStr,
        T2::Err: Display,
    {
        let (x1, x2) = x
            .split_once(" ")
            .ok_or_else(|| format!("failed to parse instruction operands on line {}", line_num))?;
        let x1: T1 = x1
            .parse()
            .map_err(|err| format!("failed to parse on line {}: {err}", line_num))?;
        let x2: T2 = x2
            .parse()
            .map_err(|err| format!("failed to parse on line {}: {err}", line_num))?;

        let instruction =
            f(x1, x2).map_err(|err| format!("failed to parse on line {}: {err}", line_num))?;

        Ok(add_unary(block, instruction))
    }
}

mod from_str {
    use std::str::FromStr;

    use crate::vm;

    pub struct VMRegisterTarget(pub vm::VMRegister);

    impl FromStr for VMRegisterTarget {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let x = extract_prefix(s, 'r');
            Ok(Self(vm::VMRegister(x.map_err(|_| {
                format!("unexpected register literal `{s}`")
            })?)))
        }
    }

    pub struct VMLocalTarget(pub vm::VMLocal);

    impl FromStr for VMLocalTarget {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let x = extract_prefix(s, '.');
            Ok(Self(vm::VMLocal(
                x.map_err(|_| format!("unexpected local literal `{s}`"))?,
            )))
        }
    }

    pub fn extract_prefix<T: FromStr>(s: &str, pattern: char) -> Result<T, ()> {
        let split = s.trim().split_once(pattern).ok_or(());
        let parsed = split.and_then(|(x, y)| y.trim().parse::<T>().map(|y| (x, y)).map_err(|_| ()));
        match parsed {
            Ok(("", x)) => Ok(x),
            _ => Err(()),
        }
    }
}
