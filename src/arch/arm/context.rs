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
        let regs = regs.as_chunks::<31>(); 
        let regs = &regs.0[0]; 

        Self {
            regs: *regs, 
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

impl LinuxContext {
    pub fn restore(&self) {
        unsafe {
            // 恢复控制寄存器和状态寄存器
            asm!(
                "
                msr sp_el0, {0}
                msr elr_el1, {1}
                msr spsr_el1, {2}
                msr ttbr0_el1, {3}
                msr ttbr1_el1, {4}
                msr tcr_el1, {5}
                msr mair_el1, {6}
                msr amair_el1, {7}
                msr sctlr_el1, {8}
                ",
                in(reg) self.sp,
                in(reg) self.pc,
                in(reg) self.spsr_el1,
                in(reg) self.ttbr0_el1,
                in(reg) self.ttbr1_el1,
                in(reg) self.tcr_el1,
                in(reg) self.mair_el1,
                in(reg) self.amair_el1,
                in(reg) self.sctlr_el1,
            );

            // 恢复通用寄存器
            asm!(
                "
                ldp x0, x1, [sp], #16
                ldp x2, x3, [sp], #16
                ldp x4, x5, [sp], #16
                ldp x6, x7, [sp], #16
                ldp x8, x9, [sp], #16
                ldp x10, x11, [sp], #16
                ldp x12, x13, [sp], #16
                ldp x14, x15, [sp], #16
                ldp x16, x17, [sp], #16
                ldp x18, x19, [sp], #16
                ldp x20, x21, [sp], #16
                ldp x22, x23, [sp], #16
                ldp x24, x25, [sp], #16
                ldp x26, x27, [sp], #16
                ldp x28, x29, [sp], #16
                ldr x30, [sp], #8
                ",
                in("x0") self.regs[0],
                in("x1") self.regs[1],
                in("x2") self.regs[2],
                in("x3") self.regs[3],
                in("x4") self.regs[4],
                in("x5") self.regs[5],
                in("x6") self.regs[6],
                in("x7") self.regs[7],
                in("x8") self.regs[8],
                in("x9") self.regs[9],
                in("x10") self.regs[10],
                in("x11") self.regs[11],
                in("x12") self.regs[12],
                in("x13") self.regs[13],
                in("x14") self.regs[14],
                in("x15") self.regs[15],
                in("x16") self.regs[16],
                in("x17") self.regs[17],
                in("x18") self.regs[18],
                in("x19") self.regs[19],
                in("x20") self.regs[20],
                in("x21") self.regs[21],
                in("x22") self.regs[22],
                in("x23") self.regs[23],
                in("x24") self.regs[24],
                in("x25") self.regs[25],
                in("x26") self.regs[26],
                in("x27") self.regs[27],
                in("x28") self.regs[28],
                in("x29") self.regs[29],
                in("x30") self.regs[30],
            );
        }
    }
}

impl GuestRegisters {
    pub fn return_to_linux(&self, linux: &LinuxContext) -> ! {
        use core::arch::asm;
        unsafe {
            asm!(
                "mov sp, {linux_sp}",
                "ldr lr, {linux_pc}",
                "mov x0, {guest_regs}",
                "add x0, x0, {guest_regs_size}",
                restore_regs_from_stack!(),
                "ldr sp, [sp, #-8]!",
                "ret",
                linux_sp = in(reg) linux.sp,
                linux_pc = in(reg) linux.pc,
                guest_regs = in(reg) self as *const _ as u64,
                guest_regs_size = const core::mem::size_of::<Self>(),
                options(noreturn),
            );
        }
    }
}