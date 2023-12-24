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
        BlockTarget(block, Some(self.blocks.len()))
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
pub struct BlockTarget(Rc<RefCell<BasicBlock>>, Option<usize>);

impl std::fmt::Debug for BlockTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.1 {
            Some(id) => f.debug_tuple("BlockTarget").field(&id).finish(),
            None => f.debug_tuple("BlockTarget").finish(),
        }
    }
}

impl BlockTarget {
    pub fn new(target: Rc<RefCell<BasicBlock>>) -> Self {
        Self(target, None)
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
    Breakpoint,
    Exit,
    Jump {
        target: BlockTarget,
    },
    JumpConditional {
        true_target: BlockTarget,
        false_target: BlockTarget,
    },
    LoadRandom {
        max: Value,
    },
}

pub mod rand {
    use std::{
        cell::{OnceCell, RefCell},
        sync::{Mutex, OnceLock},
    };

    const MODULUS: u64 = 2_147_483_647;
    const MULTIPLIER: u64 = 16_807;

    pub(crate) const F64_MULTIPLIER: f64 = 1.0 / 2_147_483_646 as f64;

    pub static INSTANCE: OnceLock<Mutex<ParkMiller>> = OnceLock::new();
    thread_local! {
        pub static FOO: OnceCell<ParkMiller> = const { OnceCell::new() };
    }

    pub struct ParkMiller {
        state: u64,
    }

    impl ParkMiller {
        pub const fn new(seed: u64) -> Self {
            Self {
                state: seed % MODULUS,
            }
        }

        pub fn thread_next(max_value: u64) -> u64 {
            let mut b: OnceCell<ParkMiller> = OnceCell::new();
            b.get_or_init(|| ParkMiller::new(123));
            match b.get_mut() {
                Some(b) => b.rand(),
                None => {
                    b.get_or_init(|| ParkMiller::new(123));
                    b.get_mut().unwrap().rand()
                }
            };
            // b.
            // let a = OnceCell::new(ParkMiller::new(123)):
            let r = FOO.with(|foo| {
                foo.get_or_init(|| ParkMiller::new(123));
                foo.get_mut().unwrap().rand()
            }) as f64
                * F64_MULTIPLIER;
            let instance = INSTANCE.get_or_init(|| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH);
                let modulo = since_epoch.unwrap().as_millis() % u64::MAX as u128;
                Mutex::new(ParkMiller::new(modulo as u64))
            });
            let r = instance.lock().unwrap().rand() as f64 * F64_MULTIPLIER;
            (max_value as f64 * r) as u64
        }
        pub fn next(max_value: u64) -> u64 {
            let instance = INSTANCE.get_or_init(|| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH);
                let modulo = since_epoch.unwrap().as_millis() % u64::MAX as u128;
                Mutex::new(ParkMiller::new(modulo as u64))
            });
            let r = instance.lock().unwrap().rand() as f64 * F64_MULTIPLIER;
            (max_value as f64 * r) as u64
        }

        pub fn rand(&mut self) -> u64 {
            self.state = self.state.wrapping_mul(MULTIPLIER) % MODULUS;
            self.state
        }
    }
}
