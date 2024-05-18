#[derive(Clone, Copy, Debug)]
pub struct NPTEntry {
    entry: u64,
}

impl NPTEntry {
    pub const fn new() -> Self {
        Self { entry: 0 }
    }

    pub fn is_unused(&self) -> bool {
        self.entry == 0
    }

    pub fn set_addr(&mut self, paddr: u64, flags: u64) {
        self.entry = (paddr & PHYS_ADDR_MASK) | (flags & !PHYS_ADDR_MASK);
    }

    pub fn clear(&mut self) {
        self.entry = 0;
    }
}

// 定义常见的页表项标志位
const PHYS_ADDR_MASK: u64 = 0x0000_ffff_ffff_f000;
const FLAG_PRESENT: u64 = 1 << 0;
const FLAG_WRITE: u64 = 1 << 1;
const FLAG_EXECUTE: u64 = 1 << 2;

use aarch64::regs::{SCTLR_EL1, VBAR_EL1};
use alloc::vec::Vec;

use crate::{cell::Cell, enclave::VcpuAccessEnclaveState, error::HvResult, memory::{addr::{phys_encrypted, virt_to_phys}, Frame, GenericPageTable, GenericPageTableImmut}, percpu::PerCpu};

use super::{EnclaveThreadState, GuestPageTableImmut, GuestRegisters, LinuxContext}; 

pub struct NestedPageTable {
    // 嵌套页表通常使用多级页表结构，在这里我们假设只用一个简单数组模拟
    entries: Vec<NPTEntry>,
}

impl GenericPageTableImmut for NestedPageTable {
    type VA = usize;

    unsafe fn from_root(root_paddr: crate::memory::PhysAddr) -> Self {
        // todo!()
        Self::new(0) 
    }

    fn root_paddr(&self) -> crate::memory::PhysAddr {
        self.entries.first().map_or(0, |e| e.entry.try_into().unwrap()) 
    }

    fn query(&self, vaddr: Self::VA) -> crate::memory::PagingResult<(crate::memory::PhysAddr, crate::memory::MemFlags, crate::memory::PageSize)> {
        todo!()
    }
}

impl GenericPageTable for NestedPageTable {
    fn new() -> Self {
        NestedPageTable::new(0) 
    }

    fn map(&mut self, region: &crate::memory::MemoryRegion<Self::VA>) -> crate::memory::PagingResult {
        todo!()
    }

    fn unmap(&mut self, region: &crate::memory::MemoryRegion<Self::VA>)
        -> crate::memory::PagingResult<Vec<(crate::memory::PhysAddr, crate::memory::PageSize)>> {
        todo!()
    }

    fn update(&mut self, region: &crate::memory::MemoryRegion<Self::VA>) -> crate::memory::PagingResult {
        todo!()
    }

    fn clone(&self) -> Self {
        todo!()
    }

    unsafe fn activate(&self) {
        todo!()
    }

    fn flush(&self, vaddr: Option<Self::VA>) {
        todo!()
    }
}

impl NestedPageTable {
    pub fn new(size: usize) -> Self {
        Self {
            entries: vec![NPTEntry::new(); size],
        }
    }

    pub fn map(&mut self, vaddr: usize, paddr: u64, flags: u64) {
        // 模拟映射虚拟地址到物理地址，这里简化为直接映射索引
        let index = vaddr / 4096; // 假设每个页大小为 4KiB
        if index < self.entries.len() {
            self.entries[index].set_addr(paddr, flags);
        }
    }

    pub fn unmap(&mut self, vaddr: usize) {
        let index = vaddr / 4096;
        if index < self.entries.len() {
            self.entries[index].clear();
        }
    }

    // pub fn query(&self, vaddr: usize) -> Option<u64> {
    //     let index = vaddr / 4096;
    //     self.entries.get(index).map(|e| e.entry)
    // }
}

pub type EnclaveNestedPageTableUnlocked = NestedPageTable;


pub trait VcpuAccessGuestState {
    // Architecture independent methods:
    fn regs(&self) -> &GuestRegisters;
    fn regs_mut(&mut self) -> &mut GuestRegisters;
    fn instr_pointer(&self) -> u64;
    fn stack_pointer(&self) -> u64;
    fn frame_pointer(&self) -> u64; 
    fn set_stack_pointer(&mut self, sp: u64);
    fn set_return_val(&mut self, ret_val: usize); 

    // Methods only available for x86 cpus:
    fn rflags(&self) -> u64;
    fn fs_base(&self) -> u64;
    fn gs_base(&self) -> u64;
    fn efer(&self) -> u64;
    fn cr(&self, cr_idx: usize) -> u64;
    fn set_cr(&mut self, cr_idx: usize, val: u64);
    fn xcr0(&self) -> u64; 
    fn set_xcr0(&mut self, val: u64); 
}

pub struct AArch64Vcpu {
    pub registers: GuestRegisters, 
}

impl VcpuAccessGuestState for AArch64Vcpu {
    fn regs(&self) -> &GuestRegisters {
        &self.registers
    }

    fn regs_mut(&mut self) -> &mut GuestRegisters {
        &mut self.registers
    }

    fn instr_pointer(&self) -> u64 {
        self.registers.pc
    }

    fn stack_pointer(&self) -> u64 {
        self.registers.sp
    }

    fn frame_pointer(&self) -> u64 {
        self.registers.x[29]  // 使用 x29 作为帧指针
    }

    fn set_stack_pointer(&mut self, sp: u64) {
        self.registers.sp = sp;
    }

    fn set_return_val(&mut self, ret_val: usize) {
        self.registers.x[0] = ret_val as u64;  // 在 AArch64 上返回值使用 x0 寄存器
    }

    // 下面的方法在 AArch64 上没有直接对应，因此提供空实现或者适当的模拟
    fn rflags(&self) -> u64 {
        // self.registers.pstate  // 使用 pstate 作为类似 rflags 的状态寄存器
        0 
    }

    fn fs_base(&self) -> u64 {
        0  // AArch64 上没有直接对应的 fs_base
    }

    fn gs_base(&self) -> u64 {
        0  // AArch64 上没有直接对应的 gs_base
    }

    fn efer(&self) -> u64 {
        0  // AArch64 上没有直接对应的 EFER 寄存器
    }

    fn cr(&self, _cr_idx: usize) -> u64 {
        0  // AArch64 上没有类似 x86 的控制寄存器 CR
    }

    fn set_cr(&mut self, _cr_idx: usize, _val: u64) {
        // AArch64 上没有类似 x86 的控制寄存器 CR
    }

    fn xcr0(&self) -> u64 {
        0  // AArch64 上没有 XCR0 寄存器
    }

    fn set_xcr0(&mut self, _val: u64) {
        // AArch64 上没有 XCR0 寄存器
    }
} 


use core::{arch::asm, convert::TryInto};

#[repr(C)]
#[derive(Debug)]
pub struct Vcpu {
    /// Save guest general registers when handle VM exits.
    guest_regs: GuestRegisters,
    /// RSP will be loaded from here when handle VM exits.
    host_stack_top: u64,
    /// host state-save area.
    host_save_area: Frame,
    /// Virtual machine control block.
    pub(super) vmcb: (),
}

impl Vcpu {
    pub fn new(linux: &LinuxContext, cell: &Cell) -> HvResult<Self> {
        // Ensure hypervisor features are enabled for ARM, e.g., by checking HCR_EL2, etc.
        // super::check_hypervisor_feature()?;

        // Performance counters control on ARM
        unsafe {
            // Disable performance counters
            // let pmcr = PMCR_EL0.read();
            // PMCR_EL0.write(pmcr & !(1 << 0));  // Disable all counters
        }

        let host_save_area = Frame::new()?;
        // Hypervisor settings for ARM can be managed through system registers like HCR_EL2

        let mut ret = Self {
            guest_regs: Default::default(),
            host_save_area,
            host_stack_top: PerCpu::from_local_base().stack_top() as _,
            vmcb: Default::default(),
        };

        ret.vmcb_setup(linux, cell);

        Ok(ret)
    }

    pub fn exit(&self, linux: &mut LinuxContext) -> HvResult {
        self.load_vmcb_guest(linux);
        // unsafe {
        //     asm!("stgi");
        //     Efer::write(Efer::read() - EferFlags::SECURE_VIRTUAL_MACHINE_ENABLE);
        //     Msr::VM_HSAVE_PA.write(0);
        // }
        info!("successed to turn off SVM.");
        Ok(())
    }

    pub fn activate_vmm(&mut self, linux: &LinuxContext) -> HvResult {
        let common_cpu_data = PerCpu::from_id(PerCpu::from_local_base().cpu_id);
        let vmcb_paddr = phys_encrypted(virt_to_phys(
            &common_cpu_data.vcpu.vmcb as *const _ as usize,
        ));
        let regs = &mut self.guest_regs; 
        // Set other registers from LinuxContext
        regs.x = linux.x; 

        regs.x[0] = vmcb_paddr as _; // General ARM register equivalent to x86's rax
        // Continue for other registers...

        unsafe {
            asm!(
                // Hypervisor mode transition on ARM
                "msr sp_el1, {0}",  // Set stack pointer for EL1
                restore_regs_from_stack!(),  // Restore registers for the VM
                "eret",  // Return from exception, equivalent to vmrun on x86
                in(reg) regs as * const _ as usize,
                options(noreturn),
            )
        }

        Ok(()) 
    }

    pub fn deactivate_vmm(&self, linux: &LinuxContext) -> HvResult {
        self.guest_regs.return_to_linux(linux)
    }

    // Additional methods, adjustments for ARM...

    pub fn inject_fault(&mut self) -> HvResult {
        // self.vmcb.inject_event(
        //     VmcbIntInfo::from(
        //         InterruptType::Exception,
        //         crate::arch::ExceptionType::GeneralProtectionFault,
        //     ),
        //     0,
        // );
        Ok(())
    }

    pub fn advance_rip(&mut self, instr_len: u8) -> HvResult {
        // self.vmcb.save.rip += instr_len as u64;
        Ok(())
    }

    pub fn rollback_rip(&mut self, instr_len: u8) -> HvResult {
        // self.vmcb.save.rip -= instr_len as u64;
        Ok(())
    }

    pub fn guest_is_privileged(&self) -> bool {
        // self.vmcb.save.cpl == 0
        false 
    }

    #[allow(dead_code)]
    pub fn in_hypercall(&self) -> bool {
        use core::convert::TryInto;
        // matches!(
            // self.vmcb.control.exit_code.try_into(),
            // Ok(SvmExitCode::VMMCALL)
        // )
        false 
    }

    pub fn guest_page_table(&self) -> GuestPageTableImmut {
        use crate::memory::addr::align_down;
        // unsafe { GuestPageTableImmut::from_root(align_down(self.vmcb.save.cr3 as _)) }
        todo!()
    }

}

// #[naked]
// unsafe extern "C" fn hypervisor_run() -> ! {
//     asm!(
//         "msr elr_el2, x0",  // Set return address in EL2
//         save_regs_to_stack!(),
//         "mov x14, x0",      // Save host x0 (analogous to rax in x86) to x14
//         "mov x15, sp",      // Save temporary SP to x15
//         "ldr x0, [sp, {0}]",  // Load Vcpu::host_stack_top to x0
//         "msr sp_el1, x0",   // Set SP for EL1 to this value
//         "bl {1}",           // Call hypervisor exit handler
//         "add sp, x15, #16", // Restore SP and skip one slot for x0
//         "push x14",         // Push saved x0 to restore it later
//         restore_regs_from_stack!(),
//         "eret",             // Return to lower exception level
//         const core::mem::size_of::<GuestRegisters>(),
//         sym crate::arch::vmm::vmexit_handler,
//         sym hypervisor_run,
//         options(noreturn),
//     )
// }

impl Vcpu {
    fn vmcb_setup(&mut self, linux: &LinuxContext, cell: &Cell) {
        self.guest_regs.x = linux.x; 
        self.guest_regs.pc = linux.pc; 
        self.guest_regs.sp = linux.sp; 
    }

    unsafe fn set_system_register(register: &str, value: u64) {
        // match register {
        //     "SCTLR_EL1" => SCTLR_EL1.write(value),
        //     "VBAR_EL1" => VBAR_EL1.write(value),
        //     _ => panic!("Unsupported system register: {}", register),
        // }
    }

    fn load_vmcb_guest(&self, linux: &mut LinuxContext) {
        return ; 
    }
}

impl VcpuAccessGuestState for Vcpu {
    fn regs(&self) -> &GuestRegisters {
        &self.guest_regs
    }

    fn regs_mut(&mut self) -> &mut GuestRegisters {
        &mut self.guest_regs
    }

    fn instr_pointer(&self) -> u64 {
        self.guest_regs.pc
    }

    fn stack_pointer(&self) -> u64 {
        self.guest_regs.sp
    }

    fn frame_pointer(&self) -> u64 {
        self.guest_regs.x[29]  // 使用 x29 作为帧指针
    }

    fn set_stack_pointer(&mut self, sp: u64) {
        self.guest_regs.sp = sp;
    }

    fn set_return_val(&mut self, ret_val: usize) {
        self.guest_regs.x[0] = ret_val as u64;  // 在 AArch64 上返回值使用 x0 寄存器
    }

    // 下面的方法在 AArch64 上没有直接对应，因此提供空实现或者适当的模拟
    fn rflags(&self) -> u64 {
        // self.guest_regs.pstate  // 使用 pstate 作为类似 rflags 的状态寄存器
        0 
    }

    fn fs_base(&self) -> u64 {
        0  // AArch64 上没有直接对应的 fs_base
    }

    fn gs_base(&self) -> u64 {
        0  // AArch64 上没有直接对应的 gs_base
    }

    fn efer(&self) -> u64 {
        0  // AArch64 上没有直接对应的 EFER 寄存器
    }

    fn cr(&self, _cr_idx: usize) -> u64 {
        0  // AArch64 上没有类似 x86 的控制寄存器 CR
    }

    fn set_cr(&mut self, _cr_idx: usize, _val: u64) {
        // AArch64 上没有类似 x86 的控制寄存器 CR
    }

    fn xcr0(&self) -> u64 {
        0  // AArch64 上没有 XCR0 寄存器
    }

    fn set_xcr0(&mut self, _val: u64) {
        // AArch64 上没有 XCR0 寄存器
    }
} 

impl VcpuAccessEnclaveState for Vcpu {
    fn load_enclave_thread_state(&self) -> HvResult<EnclaveThreadState> {
        Ok(EnclaveThreadState {
            tpidr_el0: self.guest_regs.pc, 
            elr_el1: 0, 
            spsr_el1: 0, 
            hv_page_table_root: 0, 
            page_table_root: 0, 
            // 其他状态和寄存器初始化...
        })
    }

    fn store_enclave_thread_state(
        &mut self,
        entry_ip: u64,
        state: &EnclaveThreadState,
        is_enter: bool,
    ) -> HvResult {
        // self.elr_el1 = entry_ip;
        // self.spsr_el1 = state.pstate;
        // // 设置中断和异常拦截
        // if is_enter {
        //     self.hcr_el2 |= HCR_EL2_TGE;
        // } else {
        //     self.hcr_el2 &= !HCR_EL2_TGE;
        // }

        // // 更新 TPIDR_EL0 等寄存器
        // unsafe {
        //     Msr::TPIDR_EL0.write(state.tpidr_el0);
        // }
        Ok(())
    }
}