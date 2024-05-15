use crate::error::HvResult;
use core::fmt::Debug;
use bitflags::bitflags;

bitflags! {
    #[repr(transparent)]
    pub struct EnclavePFErrorCode: u32 {
        /// #PF error code defined by ARM hardware, simplified here for clarity.
        const ARM_PF_ERROR_CODE = 0xFFFF;

        /// Additional flag to indicate page fault due to secure memory fetch.
        const SECURE_MEM_FETCH = 1 << 31;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EnclaveExceptionInfo {
    pub linux_info: ExceptionInfo,
    pub aex_excep: Option<AexException>,
}

impl EnclaveExceptionInfo {
    pub fn invalid_opcode(in_secure_mode: bool) -> Self {
        let aex_excep = if in_secure_mode {
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

    pub fn general_protection(error_code: u32) -> Self {
        let aex_excep = Some(AexException {
            vec: ExceptionType::GeneralProtectionFault,
            misc: None,
        });
        Self {
            linux_info: ExceptionInfo {
                exception_type: ExceptionType::GeneralProtectionFault,
                error_code: Some(error_code),
                cr2: None,
            },
            aex_excep,
        }
    }

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
            misc: None,
        });
        Self {
            linux_info,
            aex_excep,
        }
    }

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
    pub spsr_el2: u64,
    pub sp_el1: u64,
    pub elr_el2: u64,

    pub secure_world_state: u64,
}

impl EnclaveThreadState {
    pub fn enclave_enter(
        vcpu: &mut impl VcpuAccessEnclaveState,
        entry_ip: u64,
        sp_el1: u64,
        secure_world_state: u64,
    ) -> HvResult {
        vcpu.set_elr_el2(entry_ip);
        vcpu.set_spsr_el2(0x3C5); // EL1h mode
        vcpu.set_sp_el1(sp_el1);

        Ok(())
    }

    pub fn enclave_exit(
        vcpu: &mut impl VcpuAccessEnclaveState,
        exit_ip: u64,
        normal_world_state: &Self,
    ) -> HvResult {
        vcpu.set_elr_el2(exit_ip);
        vcpu.set_sp_el1(normal_world_state.sp_el1);
        vcpu.set_spsr_el2(normal_world_state.spsr_el2);

        Ok(())
    }
}