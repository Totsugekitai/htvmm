ENTRY(entry);

PHDRS {
    all PT_LOAD;
}

SECTIONS {
    .entry : {
        KEEP(*(.entry));
    }
    .text : {
        *(.text);
    }
    .rodata : {
        *(.rodata);
    }
    .data : {
        *(.data);
    }
}