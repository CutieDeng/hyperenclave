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

//! 该模块记录了一些常量 
//! 

pub use crate::memory::PAGE_SIZE;
pub use crate::percpu::PER_CPU_SIZE;

/// 表示当前管理程序 HyperVisor 的内存基地址，用于将 HyperVisor 与其他地址空间隔离开 
pub const HV_BASE: usize = 0xffff_ff00_0000_0000;

/// 临时映射基地址，用于临时映射内存 
pub const TEMP_MAPPING_BASE: usize = 0xffff_f000_0000_0000;

/// 表示临时映射内存的页数，只有 16 页 
pub const NUM_TEMP_PAGES: usize = 16;

/// 表示每个 CPU 本地数据结构的基地址，它的位置在所有临时映射小页之后 
pub const LOCAL_PER_CPU_BASE: usize = TEMP_MAPPING_BASE + NUM_TEMP_PAGES * PAGE_SIZE;

/// 根据是否启用了 SME 特性，设置 SME_C_BIT_OFFSET 的值
/// 
/// 该常量用于地址转换和内存加密 
pub const SME_C_BIT_OFFSET: usize = 
    if cfg!(feature = "sme") {
        1 << 47
    } else {
        0 
    }; 

/// 用于表示 HyperVisor 的堆栈大小，为 512 KB 
pub const HV_STACK_SIZE: usize = 512 * 1024; // 512 KB
