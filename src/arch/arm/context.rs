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