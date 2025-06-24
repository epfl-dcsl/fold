# Fold

Fold is a framework to create dynamic linkers in Rust.

## Quick Start

To get started, first install the dependencies:

1. Install Rust (see the [instructions](https://rust-lang.org/tools/install)).
2. Install [Just](https://github.com/casey/just) (can be installed with `cargo install just`).

Then, in the Fold folder:

Run `just run samples/hello` to run Fold with a small ELF.

You should see an "hi there".

## Build

To build `fold`, sample binaries and example linkers, just use the following command:

```sh
just build
```

## Samples

`sample` folder contain many ELF files that you can use to test your own linker.

## Test

`tests` folder contains end-to-end tests that runs fold. or example linkers, on binary samples and check correct output.

To run these tests, run:

```sh
just test
```

## Examples

[examples](./examples/) folder contains implementations of dynamic linkers using Fold.
