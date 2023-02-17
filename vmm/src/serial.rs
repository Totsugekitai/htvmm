use crate::ioapic;
use x86_64::instructions::{
    interrupts::without_interrupts,
    port::{PortReadOnly, PortWriteOnly},
};

pub const COM: u16 = if cfg!(feature = "gpd") { COM2 } else { COM1 };
pub const COM1: u16 = 0x3f8;
pub const COM2: u16 = 0x2f8;
#[allow(unused)]
pub const COM3: u16 = 0x3e8;
#[allow(unused)]
pub const COM4: u16 = 0x2e8;

const IRQ: u32 = if COM == COM1 || COM == COM3 {
    IRQ4
} else {
    IRQ3
};
const IRQ4: u32 = 4;
const IRQ3: u32 = 3;

pub unsafe fn init(com: u16) {
    without_interrupts(|| {
        // 8259 PIC Disable
        PortWriteOnly::<u8>::new(0xa1).write(0xff);
        PortWriteOnly::<u8>::new(0x21).write(0xff);
        // 16550A UART Enable
        PortWriteOnly::<u8>::new(com + 1).write(0); // disable all interrupts
        PortWriteOnly::<u8>::new(com + 3).write(0x80); // DLAB set 1
        PortWriteOnly::<u8>::new(com).write(1); // 115200 / 115200
        PortWriteOnly::<u8>::new(com + 1).write(0); // baud rate hi bytes
        PortWriteOnly::<u8>::new(com + 3).write(0x03); // DLAB set 0
        PortWriteOnly::<u8>::new(com + 4).write(0x0b); // IRQ enable
        PortWriteOnly::<u8>::new(com + 1).write(0x01); // interrupt enable

        if PortReadOnly::<u16>::new(com + 5).read() == 0xff {
            panic!();
        }

        PortReadOnly::<u16>::new(com + 2).read();
        PortReadOnly::<u16>::new(com).read();

        ioapic::enable(IRQ, 0);
    });
}

pub unsafe fn write(com: u16, c: u8) {
    while (PortReadOnly::<u16>::new(com + 5).read() & 0b10_0000) >> 5 != 1 {
        x86_64::instructions::nop();
    }

    PortWriteOnly::<u8>::new(com).write(c);
}
