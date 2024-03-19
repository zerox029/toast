# Nuke built-in rules and variables.
override MAKEFLAGS += -rR

RUST_TARGET := x86_64-unknown-none
BUILD_NAME := toast

# Default target.
.PHONY: all
all:
	cargo build --target $(RUST_TARGET)
	@cp target/$(RUST_TARGET)/debug/$(BUILD_NAME) kernel

test:
	cargo test --target $(RUST_TARGET) --no-run --target-dir target/tests
	@cp target/tests/$(RUST_TARGET)/debug/deps/$$(cd target/tests/$(RUST_TARGET)/debug/deps && find toast* -maxdepth 1 -perm -111 -type f) ./kernel

# Remove object files and the final executable.
.PHONY: clean
clean:
	@cargo clean
	@rm -rf kernel

.PHONY: distclean
distclean: clean
