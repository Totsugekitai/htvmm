ENTRY(entry);

virt = 0x40100000;
phys = 0x00100000;

PHDRS {
    all PT_LOAD;
}

SECTIONS {
    . = virt;
    head = .;

    .entry : {
        KEEP(*(.entry));
    }
}