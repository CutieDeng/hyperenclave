use core::fmt::{Debug, Formatter, Result};

// 假定 AArch64 的浮点/SIMD 寄存器状态大小
pub const FP_SIMD_STATE_SIZE: usize = 512 + 16; // 浮点和 SIMD 寄存器大小

#[repr(C)]
pub struct FpSimdStateRegion {
    state: [u8; FP_SIMD_STATE_SIZE],
    _reserved: [u8; 3369], 
}

impl FpSimdStateRegion {
    pub const fn new() -> Self {
        Self {
            state: [0; FP_SIMD_STATE_SIZE],
            _reserved: [0; 3369], 
        }
    }

    pub fn restore(&self) {
        // 模拟 AArch64 恢复 FP/SIMD 状态的逻辑
        // hahahah 
    }
}

impl Debug for FpSimdStateRegion {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_tuple("FpSimdStateRegion")
            .field(&self.state)
            .finish()
    }
}

pub use FpSimdStateRegion as XsaveRegion; 