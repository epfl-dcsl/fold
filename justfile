root_dir := justfile_directory()
target := "fold/x86_64-unknown-linux-none.json"

# Print list of commands
help:
	@just --list --unsorted

run TARGET:
	cargo +nightly run -p fold --target {{target}} -Z build-std=core,alloc -- {{TARGET}}

build:
	cargo +nightly build -p fold --target {{target}} -Z build-std=core,alloc
	@make -C samples
	@just --justfile examples/justfile


	@echo 'ARCH=x86_64' > musl/config.mak
	@sh patch-musl.sh
	@make -C musl

test:
	@just build
	cargo test -p tests

sqlite-build:
	@just --justfile sqlite-build/justfile build

release:
	cargo +nightly build -p fold --release --target {{target}} -Z build-std=core,alloc

check:
	cargo +nightly check -p fold --target {{target}} -Z build-std=core,alloc
	cargo check -p tests
	
clippy:
	cargo +nightly clippy -p fold --target {{target}} -Z build-std=core,alloc
	cargo clippy -p tests

clippy-fix:
	cargo +nightly clippy -p fold --target {{target}} -Z build-std=core,alloc --fix --allow-dirty
	cargo clippy -p tests --fix --allow-dirty

fmt:
	cargo +nightly fmt -p fold
	cargo fmt -p tests

report:
	sh reports/render.sh

doc-open:
	@just report
	cargo +nightly doc -p fold --target {{target}} -Z build-std=core,alloc --open

doc:
	@just report
	cargo +nightly doc -p fold --target {{target}} -Z build-std=core,alloc

clean:
	cargo clean
	make -C samples/justfile clean

# The following line gives highlighting on vim
# vim: set ft=make :
