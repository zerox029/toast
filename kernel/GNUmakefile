# Nuke built-in rules and variables.
override MAKEFLAGS += -rR

RUST_TARGET := x86_64-unknown-none
BUILD_NAME := limine-toast

# Default target.
.PHONY: all
all:
	cargo build --target $(RUST_TARGET)
	cp target/$(RUST_TARGET)/debug/$(BUILD_NAME) kernel

# Remove object files and the final executable.
.PHONY: clean
clean:
	cargo clean
	rm -rf kernel

.PHONY: distclean
distclean: clean
