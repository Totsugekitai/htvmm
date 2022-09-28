# HTVMM

A toy hypervisor written in Rust.

## init

```
sudo apt install ovmf qemu-system-x86
make init
```

## build

### loader

```
make build-loader
```

### vmm

```
make build-vmm
```

## release build

Please add shell variable `RELEASE=1` to make command.

```
RELEASE=1 make build
```

## run in QEMU

```
make run
```

If you want to debug, use GDB.

```
$ gdb
(gdb) target remote :1234
```
