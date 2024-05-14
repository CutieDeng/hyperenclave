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

use crate::percpu::PerCpu;

use core::arch::asm; 

#[cfg(target_arch = "x86_64")]
unsafe extern "sysv64" fn switch_stack(cpu_id: usize, linux_sp: usize) -> i32 {
    let cpu_data = PerCpu::from_id(cpu_id);
    let hv_sp = cpu_data.stack_top();
    let mut ret;
    asm!("
        mov rcx, rsp
        mov rsp, {0}
        push rcx
        call {1}
        pop rsp",
        in(reg) hv_sp,
        sym crate::entry,
        in("rdi") cpu_id,
        in("rsi") linux_sp,
        lateout("rax") ret,
    );
    ret
}

#[cfg(target_arch = "aarch64")]
unsafe extern "sysv64" fn switch_stack(cpu_id: usize, linux_sp: usize) -> i32 {
    let cpu_data = PerCpu::from_id(cpu_id);
    let hv_sp = cpu_data.stack_top();
    let mut ret;
    asm!("
        mov rcx, rsp
        mov rsp, {0}
        push rcx
        call {1}
        pop rsp",
        in(reg) hv_sp,
        sym crate::entry,
        in("rdi") cpu_id,
        in("rsi") linux_sp,
        lateout("rax") ret,
    );
    ret
}


/// HyperEnclave 应用的真正入口点
#[naked]
#[no_mangle]
pub unsafe extern "C" fn arch_entry(_cpu_id: usize) -> i32 {
    #[cfg(target_arch = "x86_64")] 
    asm!("
        // rip is pushed
        cli
        push rbp
        push rbx
        push r12
        push r13
        push r14
        push r15

        mov rsi, rsp
        call {0}

        pop r15
        pop r14
        pop r13
        pop r12
        pop rbx
        pop rbp
        ret
        // rip will pop when return",
        sym switch_stack,
        options(noreturn),
    )
    #[cfg(target_arch = "aarch64")] 
    asm!(
        "
        // x30 (link register) is automatically pushed to the stack
        mrs x0, SPSR_EL1
        mrs x1, ELR_EL1

        // Disable interrupts
        msr DAIFSet, #0xf

        // Push registers
        stp x29, x30, [sp, #-16]!
        stp x27, x28, [sp, #-16]!
        stp x25, x26, [sp, #-16]!
        stp x23, x24, [sp, #-16]!
        stp x21, x22, [sp, #-16]!
        stp x19, x20, [sp, #-16]!
        stp x17, x18, [sp, #-16]!
        stp x15, x16, [sp, #-16]!
        stp x13, x14, [sp, #-16]!
        stp x11, x12, [sp, #-16]!
        stp x9, x10, [sp, #-16]!
        stp x7, x8, [sp, #-16]!
        stp x5, x6, [sp, #-16]!
        stp x3, x4, [sp, #-16]!
        stp x1, x2, [sp, #-16]!
        stp xzr, x0, [sp, #-16]!

        mov x0, sp
        bl {0}

        // Pop registers
        ldp xzr, x0, [sp], #16
        ldp x1, x2, [sp], #16
        ldp x3, x4, [sp], #16
        ldp x5, x6, [sp], #16
        ldp x7, x8, [sp], #16
        ldp x9, x10, [sp], #16
        ldp x11, x12, [sp], #16
        ldp x13, x14, [sp], #16
        ldp x15, x16, [sp], #16
        ldp x17, x18, [sp], #16
        ldp x19, x20, [sp], #16
        ldp x21, x22, [sp], #16
        ldp x23, x24, [sp], #16
        ldp x25, x26, [sp], #16
        ldp x27, x28, [sp], #16
        ldp x29, x30, [sp], #16

        // Restore interrupts
        msr DAIFClr, #0xf

        ret
        ",
        sym switch_stack,
        options(noreturn),
    )
}
