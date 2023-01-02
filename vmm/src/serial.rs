use crate::ioapic;
use x86_64::instructions::{
    interrupts::without_interrupts,
    port::{PortReadOnly, PortWriteOnly},
};

pub const COM: u16 = if cfg!(feature = "gpd") { COM2 } else { COM1 };
pub const COM1: u16 = 0x3f8;
pub const COM2: u16 = 0x2f8;
pub const COM3: u16 = 0x3e8;
pub const COM4: u16 = 0x2e8;

const IRQ_COM1: u32 = 4;
const IRQ_COM2: u32 = 3;

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

        ioapic::enable(IRQ_COM1, 0);
    });
}

pub unsafe fn write(com: u16, c: u8) {
    if !cfg!(feature = "qemu") {
        while PortReadOnly::<u16>::new(com + 5).read() & 0x20 != 0x20 {
            x86_64::instructions::nop();
        }
    }
    PortWriteOnly::<u8>::new(com).write(c);
}
