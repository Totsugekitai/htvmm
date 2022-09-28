SHELL := bash
.RECIPEPREFIX = >
.ONESHELL:
MAKEFLAGS += --no-builtin-rules --no-builtin-variables

export RELEASE ?=
build_mode := $(if $(RELEASE),release,debug)

export RUSTFLAGS = -Z emit-stack-sizes
CARGOFLAGS += $(if $(RELEASE),--release,)

export QEMU ?= qemu-system-x86_64
QEMUFLAGS := -s \
-drive if=pflash,format=raw,readonly,file=tools/ovmf/OVMF_CODE.fd \
-drive if=pflash,format=raw,file=tools/ovmf/OVMF_VARS.fd \
-drive if=ide,file=fat:rw:image,index=0,media=disk \
#-enable-kvm -cpu kvm64,+svm

.PHONY: default
default: build

.PHONY: build-vmm
build-vmm:
> cd vmm; cargo build $(CARGOFLAGS)

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

.PHONY: run
run:
> cp vmm/target/htvmm/$(build_mode)/htvmm.elf image/htvmm.elf
> cp loader/target/x86_64-unknown-uefi/$(build_mode)/htloader.efi image/EFI/BOOT/BOOTX64.EFI
> $(QEMU) $(QEMUFLAGS)

.PHONY: init
init:
> mkdir -p tools/ovmf && \
cp `find /usr -type f -name "OVMF_CODE.fd" 2> /dev/null | head -n 1` tools/ovmf && \
cp `find /usr -type f -name "OVMF_VARS.fd" 2> /dev/null | head -n 1` tools/ovmf
> mkdir -p image/EFI/BOOT
