#[derive(Debug)]
pub struct LinuxContext {
    pub sp: u64,    // 栈指针
    pub pc: u64,    // 程序计数器

    pub x29: u64,   // 帧指针
    pub x30: u64,   // 链接寄存器
    pub x28: u64,
    pub x27: u64,
    pub x26: u64,
    pub x25: u64,
    pub x24: u64,
    pub x23: u64,
    pub x22: u64,
    pub x21: u64,
    pub x20: u64,
    pub x19: u64,
    pub x18: u64,
    pub x17: u64,
    pub x16: u64,
    pub x15: u64,
    pub x14: u64,
    pub x13: u64,
    pub x12: u64,
    pub x11: u64,
    pub x10: u64,
    pub x9: u64,
    pub x8: u64,
    pub x7: u64,
    pub x6: u64,
    pub x5: u64,
    pub x4: u64,
    pub x3: u64,
    pub x2: u64,
    pub x1: u64,
    pub x0: u64,

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
            x29: 0,
            x30: 0,
            x28: 0,
            x27: 0,
            x26: 0,
            x25: 0,
            x24: 0,
            x23: 0,
            x22: 0,
            x21: 0,
            x20: 0,
            x19: 0,
            x18: 0,
            x17: 0,
            x16: 0,
            x15: 0,
            x14: 0,
            x13: 0,
            x12: 0,
            x11: 0,
            x10: 0,
            x9: 0,
            x8: 0,
            x7: 0,
            x6: 0,
            x5: 0,
            x4: 0,
            x3: 0,
            x2: 0,
            x1: 0,
            x0: 0,
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