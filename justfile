target := "x86_64-unknown-linux-none.json"

# Print list of commands
help:
	@just --list --unsorted

run:
	cargo +nightly run --target {{target}} -Z build-std=core,alloc -- hello-loader

build:
	cargo +nightly build --target {{target}} -Z build-std=core,alloc
	@just --justfile samples/justfile


	@echo 'ARCH=x86_64' > musl/config.mak
	@(cd musl && git apply ../musl.patch || true)
	@make -C musl

check:
	cargo +nightly check --target {{target}} -Z build-std=core,alloc
	
clippy:
	cargo +nightly clippy --target {{target}} -Z build-std=core,alloc

clippy-fix:
	cargo +nightly clippy --target {{target}} -Z build-std=core,alloc --fix --allow-dirty

fmt:
	cargo +nightly fmt

doc:
	cargo +nightly doc --target {{target}} -Z build-std=core,alloc --open

clean:
	cargo clean
	@just --justfile samples/justfile clean

# The following line gives highlighting on vim
# vim: set ft=make :
