pub mod ExceptionType {

    pub const UndefinedInstruction: u8 = 35; 
    pub const DataAbort: u8 = 36; 

    pub const SyncException: u8 = 0;
    pub const IRQ: u8 = 1;
    pub const FIQ: u8 = 2;
    pub const SError: u8 = 3;

    // 特定同步异常
    pub const DataAbortLowerEL: u8 = 4;
    pub const DataAbortCurrentEL: u8 = 5;
    pub const PCAlignmentFault: u8 = 6;
    pub const SPAlignmentFault: u8 = 7;
    pub const InstructionAbortLowerEL: u8 = 8;
    pub const InstructionAbortCurrentEL: u8 = 9;

    // 虚拟化异常
    // pub const VirtualizationException: u8 = 10;

    pub const DivideError: u8 = 0;
    pub const Debug: u8 = 1;
    pub const NonMaskableInterrupt: u8 = 2;
    pub const Breakpoint: u8 = 3;
    pub const Overflow: u8 = 4;
    pub const BoundRangeExceeded: u8 = 5;
    pub const InvalidOpcode: u8 = 6;
    pub const DeviceNotAvailable: u8 = 7;
    pub const DoubleFault: u8 = 8;
    pub const CoprocessorSegmentOverrun: u8 = 9;
    pub const InvalidTSS: u8 = 10;
    pub const SegmentNotPresent: u8 = 11;
    pub const StackSegmentFault: u8 = 12;
    pub const GeneralProtectionFault: u8 = 13;
    pub const PageFault: u8 = 14;
    pub const FloatingPointException: u8 = 16;
    pub const AlignmentCheck: u8 = 17;
    pub const MachineCheck: u8 = 18;
    pub const SIMDFloatingPointException: u8 = 19;
    pub const VirtualizationException: u8 = 20;
    pub const ControlProtectionException: u8 = 21;
    pub const SecurityException: u8 = 30;

    pub const IrqStart: u8 = 32;
    pub const IrqEnd: u8 = 255;
}

use bitflags::bitflags;

bitflags! {
    /// Describes a page fault error code for AArch64.
    #[repr(transparent)]
    pub struct PageFaultErrorCode: u32 {
        const PROTECTION_VIOLATION = 1 << 0;
        const CAUSED_BY_WRITE = 1 << 1;
        const USER_MODE = 1 << 2;
        const MALFORMED_TABLE = 1 << 3;
        const INSTRUCTION_FETCH = 1 << 4;
    }
}

// #[derive(Copy, Clone, Debug)]
// pub struct ExceptionInfo {
//     pub exception_type: u8,
//     pub error_code: Option<u32>,
//     pub fault_address: Option<u64>,
// }

#[derive(Copy, Clone, Debug)]
pub struct ExceptionInfo {
    pub exception_type: u8,
    pub error_code: Option<u32>,
    pub cr2: Option<u64>,
}

impl ExceptionInfo {
    pub fn new(exception_type: u8, error_code: Option<u32>, fault_address: Option<u64>) -> Self {
        ExceptionInfo {
            exception_type,
            error_code,
            cr2: fault_address,
        }
    }
}

use core::arch::global_asm;

use crate::arch::exception;

use super::GuestRegisters;

global_asm!(include_str!(concat!(env!("OUT_DIR"), "/exception.S")));

fn exception_handler(frame: &ExceptionFrame) {
    trace!("Exception or interrupt #{:#x}", frame.num);
    match frame.num as u8 {
        exception::ExceptionType::IRQ => handle_irq(),
        exception::ExceptionType::SError => handle_serror(),
        exception::ExceptionType::DataAbortLowerEL |
        exception::ExceptionType::DataAbortCurrentEL => {
            handle_page_fault(frame)
        },
        _ => {
            error!("{:#x?}", frame);
            panic!("Unhandled exception #{:#x}", frame.num);
        }
    }
}

fn handle_irq() {
    warn!("Unhandled exception: IRQ");
}

fn handle_serror() {
    warn!("Unhandled exception: SError");
}

fn handle_page_fault(frame: &ExceptionFrame) {
    panic!(
        "Unhandled hypervisor page fault @ {:#x?}, error_code={:#x}: {:#x?}",
        frame.rip, 
        frame.error_code, 
        frame
    );
}


#[repr(C)]
#[derive(Debug)]
pub struct ExceptionFrame {
    // Pushed by `common_exception_entry`
    regs: GuestRegisters,

    // Pushed by 'exception.S'
    num: usize,
    error_code: usize,

    // Pushed by CPU
    rip: usize,
    cs: usize,
    rflags: usize,

    rsp: usize,
    ss: usize,
}

use core::arch::asm;

#[naked]
#[no_mangle]
unsafe extern "C" fn common_exception_entry() -> ! {
    asm!(
        // 保存寄存器状态到栈上
        save_regs_to_stack!(),
        // 将栈指针 x28 (栈指针使用情况可能因实现而异) 的值移动到 x0 中
        "mov x0, sp",
        // 调用异常处理函数，x0 为参数
        "bl {exception_handler}",
        // 从栈上恢复寄存器状态
        restore_regs_from_stack!(),
        // 从栈上移除额外的信息 (ARM64 中可能需要根据异常类型和上下文调整)
        "add sp, sp, #16",
        // 从异常处理程序返回
        "eret",
        exception_handler = sym exception_handler,
        options(noreturn),
    )
}