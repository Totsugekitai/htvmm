SHELL := bash
.RECIPEPREFIX = >
.ONESHELL:
MAKEFLAGS += --no-builtin-rules --no-builtin-variables

LD = ld.lld
LDFLAGS = --no-nmagic --gc-sections --Map=vmm/htvmm.map -nostdlib --script=vmm/src/htvmm.lds

export RELEASE ?=
build_mode := $(if $(RELEASE),release,debug)

export RUSTFLAGS = -Z emit-stack-sizes
CARGOFLAGS += $(if $(RELEASE),--release,)

export QEMU ?= qemu-system-x86_64
QEMUFLAGS := -s -m 8G \
-drive if=pflash,format=raw,readonly,file=tools/ovmf/OVMF_CODE.fd \
-drive if=pflash,format=raw,file=tools/ovmf/OVMF_VARS.fd \
-drive if=ide,file=fat:rw:image,index=0,media=disk \
-monitor stdio \
#-enable-kvm -cpu kvm64,+vmx

.PHONY: default
default: build

VMM_OBJ = vmm/target/htvmm/$(build_mode)/libhtvmm.a
$(VMM_OBJ): .FORCE
> cd vmm; cargo build $(CARGOFLAGS)

.PHONY: build-vmm
build-vmm: $(VMM_OBJ)
> $(LD) $(LDFLAGS) -o vmm/htvmm.elf.$(build_mode) $(VMM_OBJ)

.PHONY: build-loader
build-loader:
> cd loader; cargo build $(CARGOFLAGS)

.PHONY: build
build: build-loader build-vmm

.PHONY: clean-vmm
clean-vmm:
> cd vmm; cargo clean

.PHONY: clean-loader
clean-loader:
> cd loader; cargo clean

.PHONY: clean
clean: clean-loader clean-vmm
> rm -f vmm/htvmm.elf.* image/htvmm.elf image/EFI/BOOT/BOOTX64.EFI

.PHONY: run
run:
> cp vmm/htvmm.elf.$(build_mode) image/htvmm.elf
> cp loader/target/x86_64-unknown-uefi/$(build_mode)/htloader.efi image/EFI/BOOT/BOOTX64.EFI
> $(QEMU) $(QEMUFLAGS)

.PHONY: init
init:
> mkdir -p tools/ovmf && \
cp `find /usr -type f -name "OVMF_CODE.fd" 2> /dev/null | head -n 1` tools/ovmf && \
cp `find /usr -type f -name "OVMF_VARS.fd" 2> /dev/null | head -n 1` tools/ovmf
> mkdir -p image/EFI/BOOT

.FORCE:
