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

use core::{arch::asm, fmt::{Debug, Formatter, Result}};
use aarch64_cpu::{asm::barrier, instructions::{isb, tlbi}, registers::{CNTHCTL_EL2, CNTVOFF_EL2, ELR_EL1, HCR_EL2, ICC_SRE_EL2, MPIDR_EL1, SCTLR_EL1, SPSR_EL1, SP_EL0, VMPIDR_EL2}, HCR_EL2_FLAGS};

use crate::arch::cpuid::CpuFeatures;
use crate::arch::segmentation::Segment;
use crate::arch::vmm::VcpuAccessGuestState;
use crate::arch::{GuestPageTableImmut, LinuxContext};
use super::context::GuestRegisters; 
use crate::cell::Cell;
use crate::error::HvResult;

#[repr(C)]
pub struct Vcpu {
    /// Save guest general registers when handle VM exits.
    guest_regs: GuestRegisters,
    /// ELR_EL1 will be loaded from here when handle VM exits.
    host_elr: u64,
}

impl Vcpu {
    pub fn new(linux: &LinuxContext, cell: &Cell) -> HvResult<Self> {
        // 检查Hypervisor特性
        super::check_hypervisor_feature()?;

        // 禁用性能监视器
        if CpuFeatures::new().perf_monitor_version_id() > 0 {
            // 禁用性能计数器，AArch64下具体实现略
        }

        // 初始化EL2寄存器
        unsafe {
            // 启用EL2
            HCR_EL2.write(HCR_EL2_FLAGS::RW);
            // 启用中断控制
            ICC_SRE_EL2.write(ICC_SRE_EL2::SRE::Enable + ICC_SRE_EL2::DFB::Disable + ICC_SRE_EL2::DIB::Disable);
            // 设置虚拟计数器偏移
            CNTVOFF_EL2.set(0);
        }

        // 设置返回值
        let ret = Self {
            guest_regs: Default::default(),
            host_elr: 0,
        };

        Ok(ret)
    }

    pub fn exit(&self, linux: &mut LinuxContext) -> HvResult {
        self.load_vcpu_guest(linux)?;
        unsafe {
            HCR_EL2.modify(HCR_EL2_FLAGS::RW::Clear);
        }
        info!("Successfully turned off EL2.");
        Ok(())
    }

    pub fn activate_vmm(&mut self, linux: &LinuxContext) -> HvResult {
        let regs = self.regs_mut();
        regs.x0 = 0;
        regs.x1 = linux.x1;
        regs.x2 = linux.x2;
        regs.x3 = linux.x3;
        regs.x4 = linux.x4;
        regs.x5 = linux.x5;
        regs.x6 = linux.x6;
        regs.x7 = linux.x7;
        regs.x8 = linux.x8;
        unsafe {
            asm!(
                "mov sp, {0}",
                "eret",
                in(reg) &self.guest_regs as * const _ as usize,
            );
        }
        // 如果成功激活不会返回
        error!("Activate hypervisor failed");
        hv_result_err!(EIO)
    }

    pub fn deactivate_vmm(&self, linux: &LinuxContext) -> HvResult {
        self.guest_regs.return_to_linux(linux)
    }

    pub fn inject_fault(&mut self) -> HvResult {
        // 在AArch64上注入故障的实现，具体实现略
        Ok(())
    }

    pub fn rollback_pc(&mut self, instr_len: u8) -> HvResult {
        ELR_EL1.set(ELR_EL1.get() - instr_len as u64);
        Ok(())
    }

    pub fn advance_pc(&mut self, instr_len: u8) -> HvResult {
        ELR_EL1.set(ELR_EL1.get() + instr_len as u64);
        Ok(())
    }

    pub fn guest_is_privileged(&self) -> bool {
        // 检查当前来宾的特权级
        // AArch64上具体实现略
        true
    }

    pub fn in_hypercall(&self) -> bool {
        // 检查当前是否在hypercall中
        // AArch64上具体实现略
        true
    }

    pub fn guest_page_table(&self) -> GuestPageTableImmut {
        // 获取来宾页表
        unsafe { GuestPageTableImmut::from_root(align_down(self.read_ttbr0_el1() as _)) }
    }
}

impl Vcpu {
    fn load_vcpu_guest(&self, linux: &mut LinuxContext) -> HvResult {
        linux.elr = ELR_EL1.get();
        linux.sp = SP_EL0.get();
        linux.sctlr = SCTLR_EL1.get();
        // 更多寄存器的恢复
        Ok(())
    }

    fn read_ttbr0_el1(&self) -> u64 {
        let val;
        unsafe {
            asm!("mrs {0}, ttbr0_el1", out(reg) val);
        }
        val
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
        // self.guest_regs.
        // ELR_EL1.get()
        self.guest_regs.pc 
    }

    fn stack_pointer(&self) -> u64 {
        self.guest_regs.sp 
        // SP_EL0.get()
    }

    fn set_stack_pointer(&mut self, sp: u64) {
        self.guest_regs.pc = sp; 
        // unsafe {
        //     SP_EL0.set(sp);
        // }
    }

    fn frame_pointer(&self) -> u64 {
        self.guest_regs.regs[29]  // x29 is the frame pointer (FP) in AArch64
    }

    fn set_return_val(&mut self, ret_val: usize) {
        self.guest_regs.regs[0] = ret_val as u64;  // x0 is the return register in AArch64
    }
}

impl Debug for Vcpu {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "Vcpu {{ guest_regs: {:?}, elr: 0x{:x}, sp: 0x{:x} }}",
               self.guest_regs, self.instr_pointer(), self.stack_pointer())
    }
}