OUTPUT_FORMAT(elf64-x86-64)
ENTRY(entry)

PHDRS {
    all PT_LOAD;
}

SECTIONS {
    .entry      : { KEEP(*(.entry*)) }
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