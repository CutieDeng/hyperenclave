use bitflags::bitflags;

use super::context; 
use context::GuestRegisters;
use crate::error::{HvError, HvResult};

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
pub mod ExceptionType {
    // ARM-specific exceptions
    pub const Sync: u8 = 1;
    pub const Irq: u8 = 2;
    pub const Fiq: u8 = 3;
    pub const SError: u8 = 4;
    pub const Svc: u8 = 5;
}

bitflags! {
    /// Describes a page fault error code.
    /// In ARM, these would be adapted to match ARM's fault status codes.
    #[repr(transparent)]
    pub struct PageFaultErrorCode: u32 {
        const ACCESS_FLAG = 1 << 0;
        const PERMISSION_FAULT = 1 << 1;
        const TRANSLATION_FAULT = 1 << 2;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ExceptionInfo {
    pub exception_type: u8,
    pub error_code: Option<u32>,
    pub fault_address: Option<u64>,
}

impl ExceptionInfo {
    pub fn new(exception_type: u8, error_code: Option<u32>, fault_address: Option<u64>) -> Self {
        ExceptionInfo {
            exception_type,
            error_code,
            fault_address,
        }
    }
}

#[derive(Debug)]
pub struct ExceptionFrame {
    // General purpose registers
    regs: GuestRegisters,

    // Exception specific information
    exception_number: usize,
    error_code: usize,

    // CPU State at exception
    elr_el1: usize,
    spsr_el1: usize,
    far_el1: usize,  // Fault Address Register
}

fn exception_handler(frame: &ExceptionFrame) {
    trace!("Exception or interrupt #{}", frame.exception_number);
    match frame.exception_number as u8 {
        ExceptionType::Irq => handle_irq(),
        ExceptionType::Sync => handle_sync_exception(frame),
        _ => {
            error!("{:?}", frame);
            panic!("Unhandled exception #{}", frame.exception_number);
        }
    }
}

fn handle_irq() {
    warn!("Unhandled exception: IRQ");
}

fn handle_sync_exception(frame: &ExceptionFrame) {
    panic!(
        "Unhandled hypervisor synchronous exception at {:#x}, error_code={:#x}: {:#x?}",
        frame.far_el1,
        frame.error_code,
        frame
    );
}