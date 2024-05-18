use bitflags::bitflags;

use super::{ExceptionInfo, ExceptionType, PageFaultErrorCode}; 

// AArch64 上的 Page Fault 错误码定义
bitflags! {
    #[repr(transparent)]
    pub struct EnclavePFErrorCode: u32 {
        // const AARCH64_PF_ERROR_CODE = 0x1F; // 假设 AArch64 的 #PF 错误码位于低 5 位
        const AARCH64_PF_ERROR_CODE = PageFaultErrorCode::all().bits(); 
        const EPCM_ATTR_MISMATCH = 1 << 15;
        const SHARED_MEM_FETCH = 1 << 31;
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EnclaveExceptionInfo {
    pub linux_info: ExceptionInfo,
    pub aex_excep: Option<ExceptionInfo>, // AArch64 不需要专门的 AexException
}

// impl EnclaveExceptionInfo {
//     pub fn page_fault_in_encl(
//         errcd_for_linux: u32,
//         errcd_for_misc: u32,
//         fault_vaddr: usize,
//     ) -> Self {
//         let fault_addr_for_linux = fault_vaddr & !0xFFF; // 向下对齐到页面
//         let linux_info = ExceptionInfo {
//             exception_type: ExceptionType::PageFault,
//             error_code: Some(errcd_for_linux),
//             cr2: Some(fault_vaddr),
//         };
//         let aex_excep = Some(ExceptionInfo {
//             exception_type: ExceptionType::PageFault,
//             error_code: Some(errcd_for_misc),
//             cr2: Some(fault_vaddr as u64),
//         });
//         Self {
//             linux_info,
//             aex_excep,
//         }
//     }

//     pub fn page_fault_out_encl(error_code: u32, fault_vaddr: usize) -> Self {
//         let linux_info = ExceptionInfo {
//             exception_type: ExceptionType::PageFault,
//             error_code: Some(error_code),
//             fault_address: Some(fault_vaddr as u64),
//         };
//         Self {
//             linux_info,
//             aex_excep: None,
//         }
//     }
// }

#[derive(Debug, Default)]
pub struct EnclaveThreadState {
    pub elr_el1: u64, // 异常返回地址
    pub spsr_el1: u64, // 保存的程序状态寄存器
    pub tpidr_el0: u64, // 线程局部存储指针
    pub hv_page_table_root: u64, // 宿主的页表根地址
    pub page_table_root: u64, // 客户的页表根地址
}

impl EnclaveExceptionInfo {
    /// Generate an "Invalid Opcode" exception information.
    /// AArch64 equivalent for invalid opcode can be an undefined instruction exception.
    pub fn invalid_opcode(in_encl_mode: bool) -> Self {
        let aex_excep = if in_encl_mode {
            Some(ExceptionInfo {
                exception_type: ExceptionType::UndefinedInstruction,
                error_code: None,
                cr2: None,
            })
        } else {
            None
        };
        Self {
            linux_info: ExceptionInfo {
                exception_type: ExceptionType::UndefinedInstruction,
                error_code: None,
                cr2: None,
            },
            aex_excep,
        }
    }

    /// General Protection Fault equivalent in AArch64 can be a Data Abort exception.
    pub fn general_protection(error_code: u32, cpu_state: &CpuState) -> Self {
        let aex_excep = if *cpu_state == CpuState::EnclaveRunning {
            Some(ExceptionInfo {
                exception_type: ExceptionType::DataAbort,
                error_code: Some(error_code),
                cr2: None,
            })
        } else {
            None
        };
        Self {
            linux_info: ExceptionInfo {
                exception_type: ExceptionType::DataAbort,
                error_code: Some(error_code),
                cr2: None,
            },
            aex_excep,
        }
    }

    /// Page fault in enclave mode for AArch64.
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
        let aex_excep = Some(ExceptionInfo {
            exception_type: ExceptionType::PageFault,
            error_code: Some(errcd_for_misc),
            cr2: Some(fault_vaddr as u64),
        });
        Self {
            linux_info,
            aex_excep,
        }
    }

    /// Page fault outside enclave mode for AArch64.
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
