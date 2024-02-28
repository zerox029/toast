arch ?= x86_64
kernel := build/kernel-$(arch).bin
iso := build/os-$(arch).iso
target ?= $(arch)-toast
rust_os := target/$(target)/debug/libtoast.a
cpu_model := Nehalem-v2
disk_img := build/toast-disk.img

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg
assembly_source_files := $(wildcard src/arch/$(arch)/*.asm)
assembly_object_files := $(patsubst src/arch/$(arch)/%.asm, \
	build/arch/$(arch)/%.o, $(assembly_source_files))

qemu_flags := -s \
			  -cpu $(cpu_model) \
			  -cdrom $(iso) \
			  -drive id=disk,file=$(disk_img),if=none \
			  -device ahci,id=ahci \
			  -device ide-hd,drive=disk,bus=ahci.0 \
			  -serial stdio

.PHONY: all clean run iso kernel

all: $(kernel)

clean:
	@rm -r build

run: $(iso)
	@qemu-system-x86_64 $(qemu_flags)

run-with-crash-info: $(iso)
	@qemu-system-x86_64 $(qemu_flags) -d int -no-reboot

debug: $(iso)
	@qemu-system-x86_64 $(qemu_flags) -S

gdb:
	gdb $(kernel) -ex "target remote :1234"

iso: $(iso)

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles

$(kernel): kernel $(rust_os) $(assembly_object_files) $(linker_script)
	@ld -n --gc-sections -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) $(rust_os) --no-warn-rwx-segments

kernel:
	@cargo build --target $(target).json --lib

kernel_test:
	@cargo build --target $(target).json --lib --cfg testing

# compile assembly files
build/arch/$(arch)/%.o: src/arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@