use core::fmt::{Debug, Formatter, Result};
use core::arch::asm; 

use libvmm::msr::Msr;
use libvmm::svm::flags::{InterruptType, VmcbCleanBits, VmcbIntInfo, VmcbTlbControl};
use libvmm::svm::{vmcb::VmcbSegment, SvmExitCode, SvmIntercept, Vmcb};
use aarch64::regs::{SCTLR_EL1, HCR_EL2, ESR_EL1};
use crate::arch::segmentation::Segment;
use crate::arch::vmm::VcpuAccessGuestState;
use crate::arch::{GuestPageTableImmut, GuestRegisters, LinuxContext};
use crate::cell::Cell;
use crate::error::HvResult;
use crate::memory::addr::{phys_encrypted, virt_to_phys};
use crate::memory::{Frame, GenericPageTableImmut};
use crate::percpu::PerCpu;

#[repr(C)]
pub struct Vcpu {
    /// Save guest general registers when handle VM exits.
    guest_regs: GuestRegisters,
    /// RSP will be loaded from here when handle VM exits.
    host_stack_top: u64,
    /// host state-save area.
    host_save_area: Frame,
    /// Virtual machine control block.
    pub(super) vmcb: Vmcb,
}

impl Vcpu {
    pub fn new(linux: &LinuxContext, cell: &Cell) -> HvResult<Self> {
        super::check_hypervisor_feature()?;

        // Disable performance counters
        unsafe {
            // TODO: Implement ARM equivalent performance counter disable code
        }

        let hcr = HCR_EL2.read();
        if hcr.contains(HCR_EL2::VM) {
            return hv_result_err!(EBUSY, "Virtualization is already enabled!");
        }
        let host_save_area = Frame::new()?;
        unsafe {
            HCR_EL2.write(hcr | HCR_EL2::VM);
            // TODO: Set VM host save area
        }
        info!("Successfully enabled virtualization.");

        // Bring SCTLR_EL1 into a well-defined state.
        unsafe {
            SCTLR_EL1.write(SCTLR_EL1::read());
        }

        let mut ret = Self {
            guest_regs: Default::default(),
            host_save_area,
            host_stack_top: PerCpu::from_local_base().stack_top() as _,
            vmcb: Default::default(),
        };
        assert_eq!(
            unsafe { (&ret.guest_regs as *const GuestRegisters).add(1) as u64 },
            &ret.host_stack_top as *const _ as u64
        );
        ret.vmcb_setup(linux, cell);

        Ok(ret)
    }

    pub fn exit(&self, linux: &mut LinuxContext) -> HvResult {
        self.load_vmcb_guest(linux);
        unsafe {
            // Disable virtualization
            HCR_EL2.write(HCR_EL2::read() & !HCR_EL2::VM);
        }
        info!("Successfully disabled virtualization.");
        Ok(())
    }

    pub fn activate_vmm(&mut self, linux: &LinuxContext) -> HvResult {
        let common_cpu_data = PerCpu::from_id(PerCpu::from_local_base().cpu_id);
        let vmcb_paddr = phys_encrypted(virt_to_phys(
            &common_cpu_data.vcpu.vmcb as *const _ as usize,
        ));
        let regs = self.regs_mut();
        regs.regs[0] = vmcb_paddr as _;
        regs.regs[1] = linux.regs[1];
        regs.regs[2] = linux.regs[2];
        regs.regs[12] = linux.regs[12];
        regs.regs[13] = linux.regs[13];
        regs.regs[14] = linux.regs[14];
        regs.regs[15] = linux.regs[15];
        unsafe {
            asm!(
                "msr elr_el1, {0}",
                restore_regs_from_stack!(),
                "hvc #0",
                in(reg) regs as * const _ as usize,
                options(noreturn),
            )
        }
    }

    pub fn deactivate_vmm(&self, linux: &LinuxContext) -> HvResult {
        self.guest_regs.return_to_linux(linux)
    }

    pub fn inject_fault(&mut self) -> HvResult {
        self.vmcb.inject_event(
            VmcbIntInfo::from(
                InterruptType::Exception,
                crate::arch::ExceptionType::GeneralProtectionFault,
            ),
            0,
        );
        Ok(())
    }

    pub fn advance_rip(&mut self, instr_len: u8) -> HvResult {
        self.vmcb.save.pc += instr_len as u64;
        Ok(())
    }

    pub fn rollback_rip(&mut self, instr_len: u8) -> HvResult {
        self.vmcb.save.pc -= instr_len as u64;
        Ok(())
    }

    pub fn guest_is_privileged(&self) -> bool {
        self.vmcb.save.cpl == 0
    }

    #[allow(dead_code)]
    pub fn in_hypercall(&self) -> bool {
        use core::convert::TryInto;
        matches!(
            self.vmcb.control.exit_code.try_into(),
            Ok(SvmExitCode::VMMCALL)
        )
    }

    pub fn guest_page_table(&self) -> GuestPageTableImmut {
        use crate::memory::addr::align_down;
        unsafe { GuestPageTableImmut::from_root(align_down(self.vmcb.save.ttbr0_el1 as _)) }
    }

}

impl Vcpu {
    fn set_vmcb_dtr(vmcb_seg: &mut VmcbSegment, base: u64, limit: u32) {
        vmcb_seg.limit = limit & 0xffff;
        vmcb_seg.base = base;
    }

    fn set_vmcb_segment(vmcb_seg: &mut VmcbSegment, seg: &Segment) {
        vmcb_seg.selector = seg.selector.bits() as u32;
        vmcb_seg.attr = seg.access_rights.as_u32(); // 假设有一个方法将访问权限转换为 ARM 的格式
        vmcb_seg.limit = seg.limit as u32;
        vmcb_seg.base = seg.base;
    }

    fn vmcb_setup(&mut self, linux: &LinuxContext, cell: &Cell) {
        self.set_cr(0, linux.cr0.bits());
        self.set_cr(4, linux.cr4.bits());
        self.set_cr(3, linux.cr3);

        let vmcb = &mut self.vmcb.save;
        Self::set_vmcb_segment(&mut vmcb.cs, &linux.cs);
        Self::set_vmcb_segment(&mut vmcb.ds, &linux.ds);
        Self::set_vmcb_segment(&mut vmcb.es, &linux.es);
        Self::set_vmcb_segment(&mut vmcb.fs, &linux.fs);
        Self::set_vmcb_segment(&mut vmcb.gs, &linux.gs);
        Self::set_vmcb_segment(&mut vmcb.tr, &linux.tss);
        Self::set_vmcb_segment(&mut vmcb.ss, &Segment::invalid());
        Self::set_vmcb_segment(&mut vmcb.ldtr, &Segment::invalid());
        Self::set_vmcb_dtr(&mut vmcb.idtr, &linux.idt);
        Self::set_vmcb_dtr(&mut vmcb.gdtr, &linux.gdt);
        vmcb.cpl = 0; // Linux runs in ring 0 before migration
        vmcb.rflags = 0x2;
        vmcb.rip = linux.rip;
        vmcb.rsp = linux.rsp;
        vmcb.rax = 0;
        vmcb.sysenter_cs = Msr::IA32_SYSENTER_CS.read();
        vmcb.sysenter_eip = Msr::IA32_SYSENTER_EIP.read();
        vmcb.sysenter_esp = Msr::IA32_SYSENTER_ESP.read();
        vmcb.star = Msr::IA32_STAR.read();
        vmcb.lstar = Msr::IA32_LSTAR.read();
        vmcb.cstar = Msr::IA32_CSTAR.read();
        vmcb.sfmask = Msr::IA32_FMASK.read();
        vmcb.kernel_gs_base = Msr::IA32_KERNEL_GSBASE.read();
        vmcb.efer = linux.efer | EferFlags::SECURE_VIRTUAL_MACHINE_ENABLE.bits(); // Make the hypervisor visible
        vmcb.g_pat = linux.pat;
        vmcb.dr7 = 0x400;
        vmcb.dr6 = 0xffff_0ff0;

        let vmcb = &mut self.vmcb.control;
        vmcb.intercept_exceptions = 0;
        vmcb.np_enable = 1;
        vmcb.guest_asid = 1; // No more than one guest owns the CPU
        vmcb.clean_bits = VmcbCleanBits::empty(); // Explicitly mark all of the state as new
        vmcb.nest_cr3 = cell.gpm.page_table().root_paddr() as _;
        vmcb.tlb_control = VmcbTlbControl::FlushAsid as _;

        self.vmcb.set_intercept(SvmIntercept::NMI, true);
        self.vmcb.set_intercept(SvmIntercept::CPUID, true);
        self.vmcb.set_intercept(SvmIntercept::SHUTDOWN, true);
        self.vmcb.set_intercept(SvmIntercept::VMRUN, true);
        self.vmcb.set_intercept(SvmIntercept::VMMCALL, true);
        self.vmcb.set_intercept(SvmIntercept::VMLOAD, true);
        self.vmcb.set_intercept(SvmIntercept::VMSAVE, true);
        self.vmcb.set_intercept(SvmIntercept::STGI, true);
        self.vmcb.set_intercept(SvmIntercept::CLGI, true);
        self.vmcb.set_intercept(SvmIntercept::SKINIT, true);
    }

    fn load_vmcb_guest(&self, linux: &mut LinuxContext) {
        let vmcb = &self.vmcb.save;
        linux.pc = vmcb.pc;
        linux.sp = vmcb.sp;
        linux.regs.copy_from_slice(&vmcb.regs);
        linux.elr_el1 = vmcb.elr_el1;
        linux.spsr_el1 = vmcb.spsr_el1;
        linux.sp_el0 = vmcb.sp_el0;
        linux.ttbr0_el1 = vmcb.ttbr0_el1;
        linux.ttbr1_el1 = vmcb.ttbr1_el1;
        linux.tcr_el1 = vmcb.tcr_el1;
        linux.mair_el1 = vmcb.mair_el1;
        linux.amair_el1 = vmcb.amair_el1;
        linux.sctlr_el1 = vmcb.sctlr_el1;
        linux.actlr_el1 = vmcb.actlr_el1;
        linux.esr_el1 = vmcb.esr_el1;
        linux.far_el1 = vmcb.far_el1;
        linux.vbar_el1 = vmcb.vbar_el1;
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
        self.vmcb.save.pc
    }

    fn stack_pointer(&self) -> u64 {
        self.vmcb.save.sp
    }

    fn set_stack_pointer(&mut self, sp: u64) {
        self.vmcb.save.sp = sp
    }

    fn rflags(&self) -> u64 {
        self.vmcb.save.spsr_el1
    }

    fn fs_base(&self) -> u64 {
        self.vmcb.save.sp_el0
    }

    fn gs_base(&self) -> u64 {
        self.vmcb.save.sp_el0
    }

    fn efer(&self) -> u64 {
        self.vmcb.save.sctlr_el1
    }

    fn cr(&self, cr_idx: usize) -> u64 {
        match cr_idx {
            0 => self.vmcb.save.sctlr_el1,
            1 => self.vmcb.save.ttbr0_el1,
            2 => self.vmcb.save.ttbr1_el1,
            _ => unreachable!(),
        }
    }

    fn set_cr(&mut self, cr_idx: usize, val: u64) {
        match cr_idx {
            0 => self.vmcb.save.sctlr_el1 = val,
            1 => self.vmcb.save.ttbr0_el1 = val,
            2 => self.vmcb.save.ttbr1_el1 = val,
            _ => unreachable!(),
        }
    }
}

impl Debug for Vcpu {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("Vcpu")
            .field("guest_regs", &self.guest_regs)
            .field("pc", &self.instr_pointer())
            .field("sp", &self.stack_pointer())
            .field("spsr_el1", &self.rflags())
            .field("sctlr_el1", &self.cr(0))
            .field("ttbr0_el1", &self.cr(1))
            .field("ttbr1_el1", &self.cr(2))
            .finish()
    }
}

#[naked]
unsafe extern "C" fn svm_run() -> ! {
    asm!(
        "hvc #0", // Hypervisor Call to switch to Guest
        save_regs_to_stack!(),
        "mov x14, x0",         // save host x0 to x14 for HVC
        "mov x15, sp",         // save temporary SP to x15
        "ldr sp, [sp, #8]",    // set SP to Vcpu::host_stack_top
        "bl {1}",
        "add sp, x15, #8",     // load temporary SP and skip one place for x0
        "str x14, [sp]",       // store saved x0 to restore x0 later
        restore_regs_from_stack!(),
        "b {2}",
        const core::mem::size_of::<GuestRegisters>(),
        sym crate::arch::vmm::vmexit_handler,
        sym svm_run,
        options(noreturn),
    )
}