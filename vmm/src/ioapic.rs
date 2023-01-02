#[repr(C)]
#[derive(Debug)]
struct IoApic {
    reg: u32,
    pad: [u32; 3],
    data: u32,
}

const IOAPIC: *mut IoApic = 0xFEC0_0000 as *mut IoApic;
const REG_TABLE: u32 = 0x10;
const T_IRQ0: u32 = 32;

pub unsafe fn write(reg: u32, data: u32) {
    let ioapic = IoApic {
        reg,
        pad: [0; 3],
        data,
    };
    core::ptr::write_volatile::<IoApic>(IOAPIC, ioapic);
}

pub fn enable(irq: u32, cpunum: u32) {
    unsafe {
        write(REG_TABLE + 2 * irq, T_IRQ0 + irq);
        write(REG_TABLE + 2 * irq + 1, cpunum << 24);
    }
}
