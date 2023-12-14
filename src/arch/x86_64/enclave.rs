// Copyright (C) 2023 Ant Group CO., Ltd. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::cpuid::CpuFeatures;
use super::exception::{ExceptionInfo, ExceptionType, PageFaultErrorCode};
use super::xsave::{XSAVE_HEADER_SIZE, XSAVE_LEGACY_REGION_SIZE, XSAVE_SYNTHETIC_STATE};
use crate::enclave::sgx::{GprSgx, MiscSgx, SgxExitInfo, SgxSecs, StateSaveArea, SSA_FRAME_SIZE};
use crate::enclave::{AexException, Enclave, VcpuAccessEnclaveState};
use crate::error::HvResult;
use crate::memory::addr::{align_down, is_aligned, GuestPhysAddr, GuestVirtAddr, HostPhysAddr};
use crate::memory::PAGE_SIZE;
use crate::percpu::CpuState;

use core::fmt::Debug;

use bitflags::bitflags;
use x86::controlregs::Xcr0;
use x86_64::registers::control::Cr4Flags;
use x86_64::registers::model_specific::EferFlags;
use x86_64::registers::rflags::RFlags;

/// Intel SDM, Volume 3, 38.7.3.ECREATE: The lower 2 bits of XFRM must be set
pub const SECS_XFRM_TEMPLATE: u64 = Xcr0::XCR0_FPU_MMX_STATE.bits() | Xcr0::XCR0_SSE_STATE.bits();

bitflags! {
    #[repr(transparent)]
    pub struct EnclavePFErrorCode: u32 {
        /// #PF error code defined by hardware.
        const X86_PF_ERROR_CODE     = PageFaultErrorCode::all().bits();

        /// If this flag is set, it indicates that the page fault is caused by enclave's EPCM attribute mismatch.
        const EPCM_ATTR_MISMATCH    = 1 << 15;

        /// If this flag is set, it indicates that the access that caused the page fault was an
        /// shared memory fetch.
        const SHARED_MEM_FETCH      = 1 << 31;
    }
}

/// Normal Linux kernel is unaware of the existence of hypervisor.
/// Hypervisor needs to share some information directly with enclave in some case.
/// (For example, when userspace APP illegally accesses normal memory in enclave mode,
/// hypervisor needs to abort its execution by sending SIGSEGV.
/// But it's hard to do so, since Normal Linux kernel regards such access as legal)
///
/// To achieve it, hypervisor needs to inject elaborate exception to normal Linux,
/// and fill the actual information in the SSA region.
///
/// `EnclaveExceptionInfo` is the structure for these two types of information.
#[derive(Copy, Clone, Debug)]
pub struct EnclaveExceptionInfo {
    /// The information of exception for normal Linux,
    /// hypervisor fills the information into the exception vector (in VMCS or VMCB).
    pub linux_info: ExceptionInfo,

    /// Exceptions generated by enclave can be in different modes.
    /// - If it is generated in non-enclave mode:
    ///     aex_excep = None.
    ///     There is no AEX for enclave (already in non-enclave mode),
    ///     so its SSA remains unmodified.
    ///
    /// - If it is generated in enclave mode:
    ///     aex_excep = Some(...).
    ///     Its contents is for the actual information for enclave,
    ///     hypervisor fills the it into SSA region.
    pub aex_excep: Option<AexException>,
}

impl EnclaveExceptionInfo {
    pub fn invalid_opcode(in_encl_mode: bool) -> Self {
        let aex_excep = if in_encl_mode {
            Some(AexException {
                vec: ExceptionType::InvalidOpcode,
                misc: None,
            })
        } else {
            None
        };
        Self {
            linux_info: ExceptionInfo {
                exception_type: ExceptionType::InvalidOpcode,
                error_code: None,
                cr2: None,
            },
            aex_excep,
        }
    }

    pub fn general_protection(error_code: u32, cpu_state: &CpuState) -> Self {
        let aex_excep = if *cpu_state == CpuState::EnclaveRunning {
            Some(AexException {
                vec: ExceptionType::GeneralProtectionFault,
                misc: Some(MiscSgx::new(0, error_code)),
            })
        } else {
            None
        };
        Self {
            linux_info: ExceptionInfo {
                exception_type: ExceptionType::GeneralProtectionFault,
                error_code: Some(error_code),
                cr2: None,
            },
            aex_excep,
        }
    }

    /// Generate `EnclaveExceptionInfo` with #PF in enclave mode.
    /// Caller is able to set the #PF's error code for Linux kernel(`errcd_for_linux`)
    /// and for enclave in Misc Region(`errcd_for_misc`).
    pub fn page_fault_in_encl(
        errcd_for_linux: u32,
        errcd_for_misc: u32,
        fault_vaddr: usize,
    ) -> Self {
        let fault_addr_for_linux = align_down(fault_vaddr);
        let linux_info = ExceptionInfo::new(
            ExceptionType::PageFault,
            Some(errcd_for_linux),
            Some(fault_addr_for_linux as u64),
        );
        let aex_excep = Some(AexException {
            vec: ExceptionType::PageFault,
            misc: Some(MiscSgx::new(fault_vaddr, errcd_for_misc)),
        });
        Self {
            linux_info,
            aex_excep,
        }
    }

    /// Generate `EnclaveExceptionInfo` with #PF in non-enclave mode.
    pub fn page_fault_out_encl(error_code: u32, fault_vaddr: usize) -> Self {
        let linux_info = ExceptionInfo::new(
            ExceptionType::PageFault,
            Some(error_code),
            Some(fault_vaddr as u64),
        );
        Self {
            linux_info,
            aex_excep: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct EnclaveThreadState {
    pub rflags: u64,
    pub fs_base: u64,
    pub gs_base: u64,
    pub xcr0: u64,

    pub hv_page_table_root: HostPhysAddr,
    pub page_table_root: GuestPhysAddr,
    pub efer: u64,
    pub idtr_base: u64,
    pub idtr_limit: u32,
}

impl EnclaveThreadState {
    fn validate_xfrm(vcpu: &mut impl VcpuAccessEnclaveState, xfrm: u64) -> HvResult {
        let cr4 = Cr4Flags::from_bits_truncate(vcpu.cr(4));
        if !cr4.contains(Cr4Flags::OSFXSR) {
            return hv_result_err!(
                EINVAL,
                "EnclaveThreadState::enclave_enter(): CR4.OSFXSR != 1"
            );
        }
        if cr4.contains(Cr4Flags::OSXSAVE) {
            if vcpu.xcr0() & xfrm != xfrm {
                return hv_result_err!(EINVAL, "EnclaveThreadState::enclave_enter(): xfrm should be subset of XCR0 if CR4.OSXSAVE = 1");
            }
        } else if xfrm != SECS_XFRM_TEMPLATE {
            return hv_result_err!(
                EINVAL,
                "EnclaveThreadState::enclave_enter(): xfrm != 3 when CR4.OSXSAVE != 0"
            );
        }
        Ok(())
    }

    pub fn enclave_enter(
        vcpu: &mut impl VcpuAccessEnclaveState,
        entry_ip: u64,
        fs_base: u64,
        gs_base: u64,
        xfrm: u64,
        cssa: u32,
        hv_page_table_root: HostPhysAddr,
        page_table_root: HostPhysAddr,
    ) -> HvResult {
        EnclaveThreadState::validate_xfrm(vcpu, xfrm)?;

        let mut rflags = vcpu.rflags();
        if cfg!(feature = "enclave_interrupt") {
            rflags |= RFlags::INTERRUPT_FLAG.bits(); // Enable IRQ
        } else {
            rflags &= !RFlags::INTERRUPT_FLAG.bits(); // Disable IRQ
        }
        // Disable syscalls in efer
        let efer = vcpu.efer() - EferFlags::SYSTEM_CALL_EXTENSIONS.bits();
        let sec_world_state = Self {
            rflags,
            fs_base,
            gs_base,
            xcr0: xfrm,
            idtr_base: 0,
            idtr_limit: 0,
            efer,
            hv_page_table_root,
            page_table_root,
        };
        vcpu.regs_mut().rax = cssa as _;
        vcpu.regs_mut().rcx = vcpu.instr_pointer();
        vcpu.store_enclave_thread_state(entry_ip, &sec_world_state, true)?;
        Ok(())
    }

    pub fn enclave_exit(
        vcpu: &mut impl VcpuAccessEnclaveState,
        exit_ip: u64,
        aep: u64,
        normal_world_state: &Self,
    ) -> HvResult {
        vcpu.store_enclave_thread_state(exit_ip, normal_world_state, false)?;
        vcpu.regs_mut().rcx = aep;
        Ok(())
    }

    pub fn enclave_aex(
        vcpu: &mut impl VcpuAccessEnclaveState,
        aex_excep: AexException,
        aep: u64,
        xfrm: u64,
        tcs_vaddr: GuestVirtAddr,
        ssa: &mut StateSaveArea,
        normal_world_state: &Self,
    ) -> HvResult {
        let regs = vcpu.regs();
        let gpr = &mut ssa.gpr;
        gpr.rax = regs.rax;
        gpr.rcx = regs.rcx;
        gpr.rdx = regs.rdx;
        gpr.rbx = regs.rbx;
        gpr.rsp = vcpu.stack_pointer();
        gpr.rbp = regs.rbp;
        gpr.rsi = regs.rsi;
        gpr.rdi = regs.rdi;
        gpr.r8 = regs.r8;
        gpr.r9 = regs.r9;
        gpr.r10 = regs.r10;
        gpr.r11 = regs.r11;
        gpr.r12 = regs.r12;
        gpr.r13 = regs.r13;
        gpr.r14 = regs.r14;
        gpr.r15 = regs.r15;
        gpr.rflags = vcpu.rflags();
        gpr.rip = vcpu.instr_pointer();
        gpr.exit_info = SgxExitInfo::from_vector(aex_excep.vec);
        gpr.fs_base = vcpu.fs_base();
        gpr.gs_base = vcpu.gs_base();

        if let Some(misc_in) = aex_excep.misc {
            let ssa_misc = &mut ssa.misc;
            ssa_misc.exinfo.maddr = misc_in.exinfo.maddr;
            ssa_misc.exinfo.errcd = misc_in.exinfo.errcd;
        }

        // Save the extended state into SSA.Xsave area.
        // Which extended state will be saved is controlled by xfrm and enclave's XCR0.
        // Such operation should be placed before `store_enclave_thread_state()`,
        // since normal world's XCR0 will be restored in `store_enclave_thread_state()`.
        let xsave_region = &mut ssa.xsave;
        xsave_region.save(xfrm);
        // Set extended feature to their init state.
        // Same as above, such operation is controlled by xfrm and enclave's XCR0,
        // it should be placed before `store_enclave_thread_state()`.
        XSAVE_SYNTHETIC_STATE.restore(xfrm);

        vcpu.store_enclave_thread_state(aep, normal_world_state, false)?;

        let regs = vcpu.regs_mut();
        *regs = Default::default(); // scrub enclave context
        regs.rax = crate::hypercall::HyperCallCode::EnclaveResume as _;
        regs.rbx = tcs_vaddr as _;
        regs.rcx = aep;
        regs.rbp = gpr.urbp;
        vcpu.set_stack_pointer(gpr.ursp);
        Ok(())
    }

    pub fn enclave_resume(
        vcpu: &mut impl VcpuAccessEnclaveState,
        xfrm: u64,
        hv_page_table_root: HostPhysAddr,
        page_table_root: HostPhysAddr,
        ssa: &StateSaveArea,
    ) -> HvResult {
        EnclaveThreadState::validate_xfrm(vcpu, xfrm)?;

        let xsave_region = &ssa.xsave;
        xsave_region.validate_at_resume(xfrm)?;

        let gpr = &ssa.gpr;
        // disable syscalls in efer
        let efer = vcpu.efer() - EferFlags::SYSTEM_CALL_EXTENSIONS.bits();
        let sec_world_state = Self {
            fs_base: gpr.fs_base,
            gs_base: gpr.gs_base,
            xcr0: xfrm,
            rflags: gpr.rflags,
            idtr_base: 0,
            idtr_limit: 0,
            efer,
            hv_page_table_root,
            page_table_root,
        };
        vcpu.store_enclave_thread_state(gpr.rip, &sec_world_state, true)?;

        // Restore the extended state into SSA.Xsave area.
        // Which extended state will be restored is controlled by xfrm and enclave's XCR0.
        // Such operation should be placed after `store_enclave_thread_state()`,
        // since enclave world's XCR0 is restored in `store_enclave_thread_state()`.
        xsave_region.restore(xfrm);

        let regs = vcpu.regs_mut();
        regs.rax = gpr.rax;
        regs.rcx = gpr.rcx;
        regs.rdx = gpr.rdx;
        regs.rbx = gpr.rbx;
        regs.rbp = gpr.rbp;
        regs.rsi = gpr.rsi;
        regs.rdi = gpr.rdi;
        regs.r8 = gpr.r8;
        regs.r9 = gpr.r9;
        regs.r10 = gpr.r10;
        regs.r11 = gpr.r11;
        regs.r12 = gpr.r12;
        regs.r13 = gpr.r13;
        regs.r14 = gpr.r14;
        regs.r15 = gpr.r15;
        vcpu.set_stack_pointer(gpr.rsp);

        Ok(())
    }
}

impl SgxSecs {
    pub fn validate(&self) -> HvResult {
        if self.size < PAGE_SIZE as u64 || !self.size.is_power_of_two() {
            return hv_result_err!(
                EINVAL,
                format!(
                    "SgxSecs::validate(): secs.size {:#x} must be power of 2",
                    self.size
                )
            );
        }

        if self.ms_buf_size == 0 || !is_aligned(self.ms_buf_size as _) {
            return hv_result_err!(
                EINVAL,
                format!(
                    "SgxSecs::validate(): invalid secs.ms_buf_size {:#x}",
                    self.ms_buf_size
                )
            );
        }

        let xfrm = self.attributes.xfrm;
        if xfrm & SECS_XFRM_TEMPLATE != SECS_XFRM_TEMPLATE {
            return hv_result_err!(
                EINVAL,
                format!(
                    "SgxSecs::validate(): invalid secs.attributes.xfrm {:#x}",
                    xfrm
                )
            );
        }

        let cpuid = CpuFeatures::new();
        // Intel SDM, Volume 3, 38.7.2.1:
        // If the processor does support XSAVE, XFRM must contain a value that would be legal if loaded into XCR0
        let xcr0_supported_bits = cpuid.xcr0_supported_bits();
        if xfrm & xcr0_supported_bits != xfrm {
            return hv_result_err!(
                EINVAL,
                format!(
                    "SgxSecs::validate(): invalid xfrm {:#x}, xfrm must contain legal value if it set to xcr0",
                    xfrm
                )
            );
        }
        // Iterate all the bits set in xfrm to get the offset and size of CPU extended state component
        // Follow the pseudo code provided by Intel SDM, Volume 3, 38.7.2.2
        let xsave_size = {
            let mut offset = XSAVE_LEGACY_REGION_SIZE + XSAVE_HEADER_SIZE;
            let mut size = 0;
            for sub_leaf in 2..=63 {
                if xfrm >> sub_leaf & 0b1 == 0b1 {
                    let (offset_res, size_res) = cpuid.xsave_state_info(sub_leaf);
                    let tmp_offset = offset_res;
                    if tmp_offset >= offset + size {
                        offset = offset_res;
                        size = size_res;
                    }
                }
            }
            offset + size
        };

        let ssa_frame_size_needed =
            xsave_size + core::mem::size_of::<MiscSgx>() + core::mem::size_of::<GprSgx>();
        let ssa_frame_size_from_user = self.ssa_frame_size as usize * PAGE_SIZE;
        if ssa_frame_size_needed > ssa_frame_size_from_user {
            return hv_result_err!(
                EINVAL,
                format!(
                    "SgxSecs::validate(): ssa_framsize {:#x} not enough",
                    self.ssa_frame_size
                )
            );
        }

        if ssa_frame_size_needed > SSA_FRAME_SIZE {
            return hv_result_err!(
                EINVAL,
                format!(
                    "SgxSecs::validate(): the max SSA_FRAM_SIZE is {:#x} now",
                    SSA_FRAME_SIZE
                )
            );
        }
        Ok(())
    }
}

impl Enclave {
    pub fn fixup_exception(
        &self,
        vec: u8,
        error_code: Option<u32>,
        fault_gvaddr: Option<usize>,
    ) -> HvResult<Option<EnclaveExceptionInfo>> {
        if vec != ExceptionType::PageFault {
            let misc = if vec == ExceptionType::GeneralProtectionFault {
                let misc_errcd = match error_code {
                    Some(misc_errcd) => misc_errcd,
                    None => {
                        return hv_result_err!(
                            EINVAL,
                            "Enclave::fixup_exception(): Bug, error_code is None for #GP"
                        )
                    }
                };
                Some(MiscSgx::new(0, misc_errcd))
            } else {
                None
            };
            return Ok(Some(EnclaveExceptionInfo {
                linux_info: ExceptionInfo::new(vec, error_code, None),
                aex_excep: Some(AexException { vec, misc }),
            }));
        }

        let fault_gvaddr = match fault_gvaddr {
            Some(gvaddr) => gvaddr as usize,
            None => {
                return hv_result_err!(
                    EINVAL,
                    "Enclave::fixup_exception(): Bug, fault addr is None for #PF"
                )
            }
        };
        let error_code = match error_code {
            Some(error_code) => error_code as u32,
            None => {
                return hv_result_err!(
                    EINVAL,
                    "Enclave::fixup_exception(): Bug, error code is None for #PF"
                )
            }
        };
        // Fix up exception for #PF.
        if fault_gvaddr == 0 {
            // Deference NULL pointer, inject #PF directly
            warn!(
                "Guest Page Fault by nullptr dereference, error_code={:#x}",
                error_code,
            );
            Ok(Some(EnclaveExceptionInfo::page_fault_in_encl(
                error_code,
                error_code,
                fault_gvaddr,
            )))
        } else if self.elrange().contains(&fault_gvaddr) {
            // Fix up #PF in elrange.
            self.fixup_pf_in_elrange(error_code, fault_gvaddr)
        } else if self.shmem().read().contains(&fault_gvaddr) {
            // #PF in shared memory, error_code in aex_excep add SHARED_MEM_FETCH bit.
            // As a result, ERESUME will sync page-table mappings for gvaddr
            // from normal page-table to enclave page-table.
            Ok(Some(EnclaveExceptionInfo::page_fault_in_encl(
                error_code,
                error_code | EnclavePFErrorCode::SHARED_MEM_FETCH.bits(),
                fault_gvaddr,
            )))
        } else {
            // Invalid memory access, inject #PF with only P and U bit set.
            // As a result, normal Linux will send SIGSEGV to userspace App.
            // Enclave is still able to get the exception's information in the SSA
            warn!(
                "Illegal Guest Page Fault @ {:#x?}, error_code={:#x}, send SIGSEGV",
                fault_gvaddr, error_code,
            );
            Ok(Some(EnclaveExceptionInfo::page_fault_in_encl(
                (PageFaultErrorCode::PROTECTION_VIOLATION | PageFaultErrorCode::USER_MODE).bits(),
                error_code,
                fault_gvaddr,
            )))
        }
    }
}
