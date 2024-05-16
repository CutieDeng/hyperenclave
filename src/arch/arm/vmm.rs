use crate::{error::HvResult, percpu::PerCpu};

pub trait VcpuAccessGuestState {
    fn regs(&self) -> &GuestRegisters;
    fn regs_mut(&mut self) -> &mut GuestRegisters;
    fn instr_pointer(&self) -> u64;
    fn stack_pointer(&self) -> u64;
    fn frame_pointer(&self) -> u64;
    fn set_stack_pointer(&mut self, sp: u64);
    fn set_return_val(&mut self, ret_val: usize);
}

#[cfg(feature = "arm")]
pub use vendor::{IoPTEntry, IoPageTable, Iommu, NPTEntry, NestedPageTable, Vcpu};

const VM_EXIT_LEN_HYPERCALL: u8 = 3;

pub(super) struct VmExit<'a> {
    pub cpu_data: &'a mut PerCpu,
}

impl VmExit<'_> {
    pub fn new() -> Self {
        Self {
            cpu_data: PerCpu::from_local_base_mut(),
        }
    }

    // Example ARM trap handling for system calls
    pub fn handle_syscall(&mut self) -> HvResult {
        use crate::syscall::{syscall, SyscallNo};
        let guest_regs = self.cpu_data.vcpu.regs_mut();
        let syscall_no = guest_regs.r7 as u32;
        match syscall_no {
            SyscallNo::Read => {
                let fd = guest_regs.r0 as i32;
                let buf = guest_regs.r1 as *mut u8;
                let count = guest_regs.r2 as usize;
                let result = syscall::handle_read(fd, buf, count);
                guest_regs.r0 = result as _;
            },
            _ => return Err(crate::error::HvError::UnsupportedOperation),
        }
        Ok(())
    }

    // Placeholder for handling system exceptions, e.g., access to secure memory
    pub fn handle_exception(&mut self) -> HvResult {
        let guest_regs = self.cpu_data.vcpu.regs();
        println!("Exception at PC {:#x}", guest_regs.pc);
        Ok(())
    }
}

pub(super) fn exception_handler() {
    let mut handler = VmExit::new();
    if let Err(err) = handler.handle_exception() {
        error!("Failed to handle exception, guest fault...\n{:?}", err);
        handler.cpu_data.fault().unwrap();
    }
}