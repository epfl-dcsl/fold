FOLD ?= $(CURDIR)/../target/x86_64-unknown-linux-none/debug/fold
EXAMPLES_DIR ?= $(CURDIR)/../examples/target/x86_64-unknown-linux-none/debug/

# Targets are split accross multiple categories, depending on the linker that they need.
# The linker must be passed in `$(CATEGORY)_LOADER`.
SYSV :=  hello-asm hello-pie hello-mov-pie hello-dl hello-c hello-args hello-bss hello-env hello-math hello-threaded hello-threaded-pic
SYSV_LOADER := $(FOLD)
TRAMP := trampoline-print
TRAMP_LOADER := $(EXAMPLES_DIR)/trampoline-linker
SECCOMP := seccomp-allowed seccomp-forbidden
SECCOMP_LOADER := $(EXAMPLES_DIR)/seccomp-linker
SECCOMP_SYM := seccomp-sym-hello-c
SECCOMP_SYM_LOADER := $(EXAMPLES_DIR)/seccomp-sym-linker

TARGETS_HOLDERS := SYSV TRAMP SECCOMP SECCOMP_SYM

TARGETS := $(foreach cat,$(TARGETS_HOLDERS), $($(cat)))

CC := musl-gcc
CFLAGS += -fPIC -g

all: $(TARGETS)
	$(foreach cat,$(TARGETS_HOLDERS), $(foreach target, $($(cat)), patchelf --set-interpreter $($(cat)_LOADER) $(target);))

libmsg.so: msg.o
	ld -shared msg.o -o libmsg.so

hello-dl: libmsg.so
hello-threaded: libcount.so
hello-threaded-pic: libcount-pic.so
trampoline-print: hello-c.c
	$(CC) $(CFLAGS) $^ -o $@
seccomp-sym-hello-c: hello-c.c
	$(CC) $(CFLAGS) $^ -o $@

%.o: %.c
	$(CC) -c $(CFLAGS) $^ -o $@
%.o: %.asm
	nasm -f elf64 $^ -o $@
%: %.o
	ld -pie $^ -o $@

lib%.so: %.o
	ld -shared $^ -o $@

test:
	@echo $(foreach var,$(TARGETS_HOLDERS),$($(var)))

clean:
	rm -f *.o *.so
	rm -f $(TARGETS)
