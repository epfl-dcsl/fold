# Fold

Fold is a framework to create dynamic linkers in Rust.

## Quick Start

To get started, first install the dependencies:

- [Rust and `cargo`](https://rust-lang.org/tools/install)
- [`just`](https://github.com/casey/just)
- `nasm`
- `patchelf`
- `gcc`
- `make` (for `sqlite3` build)
- `typst` (for rendering the [report](./report/))

See `just help` to list available commands. The most important ones are:

- `just build`
- `just test`
- `just run <TARGET>`, e.g. `just run samples/hello`.

## IDE configuration

To have full Intellisense and linter support, we recommend to use VSCode with the rust-analyzer extension. Add the following lines to `.vscode/settings.json`:

```jsonc
{
  "rust-analyzer.linkedProjects": [
    "${workspaceFolder}/Cargo.toml",
    "${workspaceFolder}/tests/Cargo.toml",
    "${workspaceFolder}/examples/emulator-linker/Cargo.toml",
    "${workspaceFolder}/examples/seccomp-linker/Cargo.toml",
    "${workspaceFolder}/examples/seccomp-sym-linker/Cargo.toml",
    "${workspaceFolder}/examples/trampoline-linker/Cargo.toml"
  ]
}

```

## Examples

The [examples](./examples/) folder contains implementations of dynamic linkers using Fold. See the [report](./report/report.pdf) for more details (if not already done, it can be built with `just report`).

### Syscall filtering

From a security perspective, it could be interesting to reduce the number of syscalls a process have access to. The `seccomp` syscall exactly do that! It uses a filter implemented as an `eBPF` program to restrict usage of syscalls. What we can do with Fold, is to call `seccomp` before jumping to the entry point of our program.

### Inter-module communication

We can push the previous syscall filter idea further. For example, we could scan the object to detect the syscalls used and then restrict the process to only this set.

### Function hooks

The goal of this example is to allow the injection of hooks before some of the dynamically linked functions. To be considered successful, these hooks should be invisible both to the program itself and to the libraries.
