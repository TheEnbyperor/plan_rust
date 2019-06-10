arch ?= x86_64
target ?= $(arch)-none
rust_os := target/$(target)/debug/libplan_rust.a
kernel := build/kernel-$(arch).bin
iso := build/os-$(arch).iso
initrd := build/initrd-$(arch).tar

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg
assembly_source_files := $(wildcard src/arch/$(arch)/*.s)
assembly_object_files := $(patsubst src/arch/$(arch)/%.s, \
	build/arch/$(arch)/%.o, $(assembly_source_files))
initrd_source_files := $(wildcard src/arch/$(arch)/initrd/*)

.PHONY: all clean run iso

all: $(kernel)

clean:
	@rm -r build target

run: $(iso)
	@qemu-system-x86_64 -cdrom $(iso) -s

iso: $(iso)

$(iso): $(kernel) $(grub_cfg) $(initrd)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(initrd) build/isofiles/boot/initrd.tar
	@cp $(grub_cfg) build/isofiles/boot/grub
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles

$(kernel): kernel $(rust_os) $(assembly_object_files) $(linker_script)
	@ld -n -T $(linker_script) -o $(kernel) $(assembly_object_files) $(rust_os)

$(initrd): $(initrd_source_files)
	@tar -cf $(initrd) -C src/arch/$(arch)/initrd $(patsubst src/arch/$(arch)/initrd/%, %, $(initrd_source_files))

kernel:
	@cargo xbuild --target $(target).json

# compile assembly files
build/arch/$(arch)/%.o: src/arch/$(arch)/%.s
	@mkdir -p $(shell dirname $@)
	@nasm -f elf64 -g $< -o $@