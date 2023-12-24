use crate::vm::{Value, VM};

use super::Jit;

pub struct Executable {
    code: mmap::MemoryMap,
}

impl Executable {
    pub fn new(jit: Jit) -> Self {
        let buffer_size = jit.assembler.len(); // Replace with the actual size
        let buffer_size = (buffer_size as f32 / mmap::MemoryMap::granularity() as f32).ceil()
            as usize
            * mmap::MemoryMap::granularity();

        pub const MAP_PRIVATE: std::os::raw::c_int = 0x0002;
        pub const MAP_ANON: std::os::raw::c_int = 0x1000;
        pub const MAP_JIT: std::os::raw::c_int = 0x0800;

        // Allocate executable memory
        let executable_memory_opts = &[
            mmap::MapOption::MapNonStandardFlags(MAP_ANON | MAP_PRIVATE | MAP_JIT),
            mmap::MapOption::MapReadable,
            mmap::MapOption::MapWritable,
            mmap::MapOption::MapExecutable,
        ];

        eprintln!("allocating executable memory block...");
        let executable_memory = mmap::MemoryMap::new(buffer_size, executable_memory_opts)
            .expect("couldn't allocate executable memory block");

        eprintln!("disabling write protections on thread...");
        // Safety: this is safe to call here, no return/error value to handle
        unsafe { libc::pthread_jit_write_protect_np(0) }

        eprintln!("copying bytecode to exec memory block...");
        assert!(
            executable_memory.len() > jit.assembler.len(),
            "buffer overflow"
        );
        // Safety: the size of this buffer is greater than the jit.assembler.len()
        unsafe { jit.copy_into(executable_memory.data()) }
        jit.dump_exec_addr(executable_memory.data());

        eprintln!("re-enabling write protections on thread...");
        // Safety: this is safe to call here, no return/error value to handle
        unsafe { libc::pthread_jit_write_protect_np(1) }

        eprintln!("copied bytecode to exec memory block");

        Self {
            code: executable_memory,
        }
    }

    pub fn run(&self, vm: &mut VM) {
        eprintln!("transmuting ptr");
        // Safety: this function will not return anything and arguments are place in x0,x1,x2... registers
        let exec_fn: fn(*const VM, *mut Value, *mut Value) =
            unsafe { std::mem::transmute(self.code.data()) };

        eprintln!("running fn ptr");

        // x0: VM& vm
        // x1: Value* registers
        // x2: Value* locals
        exec_fn(
            vm as *const VM,
            vm.registers.as_mut_ptr(),
            vm.locals.as_mut_ptr(),
        );

        eprintln!("finished running fn ptr");
    }
}
