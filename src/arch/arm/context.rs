use core::{arch::asm, convert::TryInto};

macro_rules! save_regs_to_stack {
    () => {
        "
        stp x29, x30, [sp, #-16]!
        stp x27, x28, [sp, #-16]!
        stp x25, x26, [sp, #-16]!
        stp x23, x24, [sp, #-16]!
        stp x21, x22, [sp, #-16]!
        stp x19, x20, [sp, #-16]!
        stp x17, x18, [sp, #-16]!
        stp x15, x16, [sp, #-16]!
        stp x13, x14, [sp, #-16]!
        stp x11, x12, [sp, #-16]!
        stp x9, x10, [sp, #-16]!
        stp x7, x8, [sp, #-16]!
        stp x5, x6, [sp, #-16]!
        stp x3, x4, [sp, #-16]!
        stp x1, x2, [sp, #-16]!
        str x0, [sp, #-8]!"
    };
}

macro_rules! restore_regs_from_stack {
    () => {
        "
        ldr x0, [sp], #8
        ldp x1, x2, [sp], #16
        ldp x3, x4, [sp], #16
        ldp x5, x6, [sp], #16
        ldp x7, x8, [sp], #16
        ldp x9, x10, [sp], #16
        ldp x11, x12, [sp], #16
        ldp x13, x14, [sp], #16
        ldp x15, x16, [sp], #16
        ldp x17, x18, [sp], #16
        ldp x19, x20, [sp], #16
        ldp x21, x22, [sp], #16
        ldp x23, x24, [sp], #16
        ldp x25, x26, [sp], #16
        ldp x27, x28, [sp], #16
        ldp x29, x30, [sp], #16"
    };
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct GuestRegisters {
    pub x: [u64; 31], // x0-x30
    pub sp: u64,      // Stack Pointer
    pub pc: u64,      // Program Counter
}

#[derive(Debug)]
pub struct LinuxContext {
    pub sp: u64,
    pub pc: u64,
    pub x: [u64; 31],
    pub pstate: u64, // Processor state
}

impl LinuxContext {
    pub fn load_from(linux_sp: usize) -> Self {
        let regs = unsafe { core::slice::from_raw_parts(linux_sp as *const u64, 33) }; // 31 general registers + SP + PC
        Self {
            sp: regs[31],
            pc: regs[32],
            x: regs[0..31].try_into().unwrap(),
            pstate: 0, // 需要从系统寄存器读取
        }
    }
}

impl LinuxContext {
    pub fn restore(&self) {
        unsafe {
            asm!(
                "msr elr_el1, {pc}",
                "msr sp_el0, {sp}",
                "msr spsr_el1, {pstate}",
                restore_regs_from_stack!(),
                "eret",
                pc = in(reg) self.pc,
                sp = in(reg) self.sp,
                pstate = in(reg) self.pstate,
            );
        }
    }
}

impl GuestRegisters {
    pub fn return_to_linux(&self, linux: &LinuxContext) -> ! {
        unsafe {
            asm!(
                "mov sp, {linux_sp}",
                "ldr x30, {linux_pc}",
                save_regs_to_stack!(),
                "mov x30, sp",
                "mov sp, {guest_regs}",
                restore_regs_from_stack!(),
                "mov sp, x30",
                "br x30",
                linux_sp = in(reg) linux.sp,
                linux_pc = in(reg) linux.pc,
                guest_regs = in(reg) self,
                options(noreturn),
            );
        }
    }
}

