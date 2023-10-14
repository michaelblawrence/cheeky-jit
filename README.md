# cheeky-jit - a bytecode JIT compiler for AArch64

cheeky-jit is a small-scale bytecode JIT compiler written in Rust. Please note that this is purely a toy project, so don't take it too seriously! Inspired by Andreas Kling, whose [C++ proof-of-concept video](https://www.youtube.com/watch?v=8mxubNQC5O8) is well worth a watch! cheeky-jit aims to replicate his work on Apple Silicon's AArch64 architecture.

With cheeky-jit, you can compile and execute bytecode programs efficiently on your Apple Silicon-powered device. Did I say efficiently? I meant eventually, my bad.

## Table of Contents
- [Introduction](#introduction)
- [Features](#features)
- [Getting Started](#getting-started)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

## Introduction

This project implements a bytecode JIT compiler for Apple Silicon AArch64 architecture. It includes a virtual machine (VM) that executes bytecode programs either JIT compiled or interpreted.

## Features

- Just-In-Time Compilation: This project uses a JIT compiler to dynamically compile and execute bytecode programs, maximizing execution speed.
- Virtual Machine: A virtual machine is provided to execute compiled or interpreted code.
- Sample Program: A sample bytecode program is included to help you get started quickly.

## Getting Started

To get started with this project, follow these steps:

1. Clone this repository to your local machine:

   ```shell
   git clone https://github.com/michaelblawrence/cheeky-jit.git
   ```

2. Navigate to the project directory:

   ```shell
   cd cheeky-jit
   ```

3. Build the project using Rust:

   ```shell
   cargo build --release
   ```

4. Run the project with one of the following options:

## Usage

This project provides three different modes of execution:

### 1. JIT Compilation and Execution

To run the program with JIT compilation and execution, simply execute the project without any command-line arguments:

```shell
cd ./target/release
./cheekyjit
```

This will compile the sample program and execute it using the JIT compiler.

### 2. No JIT Compilation

If you want to run the program interpreted, without JIT compilation, use the `--no-jit` flag:

```shell
./cheekyjit --no-jit
```

This will interpret the sample program without JIT compilation.

### 3. JIT Compilation with a Dummy Execution

To run the program without JIT compilation but with a dummy execution, use the `--nop` flag:

```shell
./cheekyjit --nop
```

This will still perform JIT compilation but will immediately return without performing any computation.

## Contributing

If you're interested in contributing to `cheeky-jit`, please follow standard Rust community guidelines and submit a PR on our repository.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE.txt) file for details.