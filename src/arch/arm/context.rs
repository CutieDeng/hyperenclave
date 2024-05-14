#[derive(Debug)]
pub struct LinuxContext {
    pub sp: u64,    // 栈指针
    pub pc: u64,    // 程序计数器

    pub regs: [u64; 31],  // 通用寄存器 x0..x30

    pub elr_el1: u64,   // 异常返回寄存器
    pub spsr_el1: u64,  // 保存的程序状态寄存器
    pub sp_el0: u64,    // EL0 的栈指针

    pub ttbr0_el1: u64, // 变换基址寄存器 0
    pub ttbr1_el1: u64, // 变换基址寄存器 1
    pub tcr_el1: u64,   // 变换控制寄存器
    pub mair_el1: u64,  // 内存属性归属寄存器
    pub amair_el1: u64, // 异常级别内存属性归属寄存器
    pub sctlr_el1: u64, // 系统控制寄存器
    pub actlr_el1: u64, // 辅助控制寄存器
    pub esr_el1: u64,   // 异常综合寄存器
    pub far_el1: u64,   // 错误地址寄存器

    pub vbar_el1: u64,  // 异常向量基址寄存器
}

impl LinuxContext {
    pub fn new() -> Self {
        LinuxContext {
            sp: 0,
            pc: 0,
            regs: [0; 31],
            elr_el1: 0,
            spsr_el1: 0,
            sp_el0: 0,
            ttbr0_el1: 0,
            ttbr1_el1: 0,
            tcr_el1: 0,
            mair_el1: 0,
            amair_el1: 0,
            sctlr_el1: 0,
            actlr_el1: 0,
            esr_el1: 0,
            far_el1: 0,
            vbar_el1: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct GuestRegisters {
    pub regs: [u64; 31],  // 通用寄存器 x0..x30
    pub sp: u64,          // 栈指针
    pub pc: u64,          // 程序计数器
}

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
        str x0, [sp, #-8]!
        "
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
        ldp x29, x30, [sp], #16
        "
    };
}

impl LinuxContext {
    // 从 linux 栈指针读出 LinuxContext 内容
    pub fn load_from(linux_sp: usize) -> Self {
        let regs = unsafe { core::slice::from_raw_parts(linux_sp as *const u64, 31) };

        Self {
            regs: [
                regs[0], regs[1], regs[2], regs[3], regs[4], regs[5], regs[6],
                regs[7], regs[8], regs[9], regs[10], regs[11], regs[12], regs[13],
                regs[14], regs[15], regs[16], regs[17], regs[18], regs[19], regs[20],
                regs[21], regs[22], regs[23], regs[24], regs[25], regs[26], regs[27],
                regs[28], regs[29], regs[30],
            ],
            sp: unsafe { core::ptr::read((linux_sp + 31 * 8) as *const u64) },
            pc: unsafe { core::ptr::read((linux_sp + 32 * 8) as *const u64) },
            spsr_el1: unsafe { core::ptr::read((linux_sp + 33 * 8) as *const u64) },
            elr_el1: unsafe { core::ptr::read((linux_sp + 34 * 8) as *const u64) },
            ttbr0_el1: unsafe { core::ptr::read((linux_sp + 35 * 8) as *const u64) },
            ttbr1_el1: unsafe { core::ptr::read((linux_sp + 36 * 8) as *const u64) },
            tcr_el1: unsafe { core::ptr::read((linux_sp + 37 * 8) as *const u64) },
            mair_el1: unsafe { core::ptr::read((linux_sp + 38 * 8) as *const u64) },
            amair_el1: unsafe { core::ptr::read((linux_sp + 39 * 8) as *const u64) },
            sctlr_el1: unsafe { core::ptr::read((linux_sp + 40 * 8) as *const u64) },
        }
    }
}