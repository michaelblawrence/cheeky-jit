use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Default)]
pub struct VM {
    pub registers: Vec<Value>,
    pub locals: Vec<Value>,
}

impl VM {
    pub fn new(register_count: usize, local_count: usize) -> Self {
        assert!(register_count > 0);
        Self {
            registers: vec![Value(0); register_count],
            locals: vec![Value(0); local_count],
        }
    }

    pub fn accum_reg(&self) -> &Value {
        &self.registers[0]
    }

    pub fn accum_reg_mut(&mut self) -> &mut Value {
        &mut self.registers[0]
    }

    pub fn dump(&self) {
        eprintln!("Registers:");
        for (i, register) in self.registers.iter().enumerate() {
            eprintln!("    [{}] {:?}", i, register);
        }
        eprintln!("Locals:");
        for (i, local) in self.locals.iter().enumerate() {
            eprintln!("    [{}] {:?}", i, local);
        }
        eprintln!("");
    }
}

#[derive(Debug, Default)]
pub struct Program {
    pub blocks: Vec<Rc<RefCell<BasicBlock>>>,
}

impl Program {
    pub fn make_block(&mut self) -> BlockTarget {
        let block = BasicBlock::default();
        let block = Rc::new(RefCell::new(block));
        self.blocks.push(block.clone());
        BlockTarget(block)
    }

    pub fn dump(&self) {
        for (i, block) in self.blocks.iter().enumerate() {
            eprintln!("Block {}:", i + 1);
            block.borrow().dump();
        }
        eprintln!("");
    }
}

#[derive(Clone)]
pub struct BlockTarget(Rc<RefCell<BasicBlock>>);

impl std::fmt::Debug for BlockTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BlockTarget").finish()
    }
}

impl BlockTarget {
    pub fn new(target: Rc<RefCell<BasicBlock>>) -> Self {
        Self(target)
    }
    pub fn append(&self, instruction: Instruction) {
        let instruction = Rc::new(RefCell::new(instruction));
        self.0.borrow_mut().instructions.push(instruction);
    }
    pub fn insert_jump_marker(&self, post_jmp_position: usize) {
        self.0
            .borrow_mut()
            .jumps_to_here
            .push(post_jmp_position - 4);
    }
    pub fn instruction(&self, index: usize) -> Instruction {
        self.0.borrow().instructions[index].clone().borrow().clone()
    }
    pub fn len(&self) -> usize {
        self.0.borrow().instructions.len()
    }
}

#[derive(Debug, Default)]
pub struct BasicBlock {
    pub instructions: Vec<Rc<RefCell<Instruction>>>,
    pub jumps_to_here: Vec<usize>,
    pub offset: usize,
}

impl BasicBlock {
    pub fn dump(&self) {
        for (i, instruction) in self.instructions.iter().enumerate() {
            eprintln!("    [{}] {:?}", i, instruction.borrow());
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Value(pub u64);

#[derive(Debug, Clone, Copy)]
pub struct VMRegister(pub usize);

#[derive(Debug, Clone, Copy)]
pub struct VMLocal(pub usize);

#[derive(Debug, Clone)]
pub enum Instruction {
    LoadImmediate {
        value: Value,
    },
    Load {
        reg: VMRegister,
    },
    Store {
        reg: VMRegister,
    },
    SetLocal {
        local: VMLocal,
    },
    GetLocal {
        local: VMLocal,
    },
    Increment,
    LessThan {
        lhs: VMRegister,
    },
    Exit,
    Jump {
        target: BlockTarget,
    },
    JumpConditional {
        true_target: BlockTarget,
        false_target: BlockTarget,
    },
}
