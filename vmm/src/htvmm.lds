OUTPUT_FORMAT(elf64-x86-64)
ENTRY(entry)

PHDRS {
    all PT_LOAD;
}

base = 0x100000000;
SIZE_HEADER = 0x1000;

SECTIONS {
    . = base;
    . += SIZE_HEADER;
    .entry      : { *(.entry*) }
    .text       : { *(.text*) }
    .rodata     : { *(.rodata*) }
    .data       : { *(.data*) }
    .bss        : {
        . = ALIGN(8);
        __bss = .;
        *(.bss*)
        . = ALIGN(8);
        __bss_end = .;
    }
}