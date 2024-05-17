use bitflags::bitflags;

bitflags! {
    #[repr(transparent)]
    pub struct EnclavePFErrorCode: u32 {
        /// 标准的 AArch64 页错误码
        const AARCH64_PF_ERROR_CODE = 0x1F; // 假设 AArch64 的 #PF 错误码位于低 5 位

        /// 如果设置了这个标志，表示页错误是由于 enclave 的权限或属性不匹配引起的
        const EPCM_ATTR_MISMATCH    = 1 << 15;

        /// 如果设置了这个标志，表示引起页错误的访问是对共享内存的读取
        const SHARED_MEM_FETCH      = 1 << 31;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EnclaveExceptionInfo {
    /// 为普通 Linux 提供的异常信息
    pub linux_info: ExceptionInfo,

    /// 由 enclave 产生的异常信息
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
                fault_address: None,
            },
            aex_excep,
        }
    }

    pub fn general_protection(error_code: u32) -> Self {
        let aex_excep = Some(AexException {
            vec: ExceptionType::GeneralProtectionFault,
            misc: Some(MiscSgx::new(0, error_code)),
        });
        Self {
            linux_info: ExceptionInfo {
                exception_type: ExceptionType::GeneralProtectionFault,
                error_code: Some(error_code),
                fault_address: None,
            },
            aex_excep,
        }
    }

    pub fn page_fault_in_encl(
        errcd_for_linux: u32,
        errcd_for_misc: u32,
        fault_vaddr: usize,
    ) -> Self {
        let fault_addr_for_linux = fault_vaddr & !0xFFF; // 向下对齐到页面
        let linux_info = ExceptionInfo {
            exception_type: ExceptionType::PageFault,
            error_code: Some(errcd_for_linux),
            fault_address: Some(fault_addr_for_linux as u64),
        };
        let aex_excep = Some(AexException {
            vec: ExceptionType::PageFault,
            misc: Some(MiscSgx::new(fault_vaddr, errcd_for_misc)),
        });
        Self {
            linux_info,
            aex_excep,
        }
    }

    pub fn page_fault_out_encl(error_code: u32, fault_vaddr: usize) -> Self {
        let linux_info = ExceptionInfo {
            exception_type: ExceptionType::PageFault,
            error_code: Some(error_code),
            fault_address: Some(fault_vaddr as u64),
        };
        Self {
            linux_info,
            aex_excep: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct EnclaveThreadState {
    pub elr_el1: u64, // 异常返回地址
    pub spsr_el1: u64, // 保存的程序状态寄存器
    pub tpidr_el0: u64, // 线程局部存储指针

    pub hv_page_table_root: u64, // 宿主的页表根地址
    pub page_table_root: u64, // 客户的页表根地址
}

impl EnclaveThreadState {
    pub fn enclave_enter(
        vcpu: &mut impl VcpuAccessEnclaveState,
        entry_ip: u64,
        tpidr_el0: u64,
        xfrm: u64,
    ) -> HvResult {
        vcpu.set_elr_el1(entry_ip);
        vcpu.set_spsr_el1(0x340); // 设置为 AArch64 的用户态
        vcpu.set_tpidr_el0(tpidr_el0);

        let sec_world_state = Self {
            elr_el1: entry_ip,
            spsr_el1: 0x340,
            tpidr_el0,
            hv_page_table_root: vcpu.get_hv_page_table_root(),
            page_table_root: vcpu.get_guest_page_table_root(),
        };
        vcpu.store_enclave_thread_state(&sec_world_state)?;
        Ok(())
    }

    pub fn enclave_exit(
        vcpu: &mut impl VcpuAccessEnclaveState,
        normal_world_state: &Self,
    ) -> HvResult {
        vcpu.store_enclave_thread_state(normal_world_state)?;
        Ok(())
    }
}