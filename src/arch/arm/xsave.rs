use core::arch::asm;
use core::fmt::{Debug, Formatter, Result};
use crate::error::HvResult;

pub static FP_SIMD_STATE: FpSimdState = FpSimdState::new();

#[repr(align(16))]
pub struct FpSimdState([u8; FP_SIMD_REGION_SIZE]);

pub const FP_SIMD_REGION_SIZE: usize = 512; // Adjust as per your SIMD register count and size

impl FpSimdState {
    pub const fn new() -> Self {
        Self([0; FP_SIMD_REGION_SIZE])
    }

    pub fn save(&mut self) {
        unsafe {
            // Example for saving D0-D31 registers in AArch64 using intrinsics or direct assembly
            asm!(
                "stp q0, q1, [{0}]",
                "stp q2, q3, [{0}, #32]",
                // Continue for all registers you want to save
                in(reg) self.0.as_mut_ptr(),
                options(nostack, preserves_flags)
            );
        }
    }

    pub fn restore(&self) {
        unsafe {
            asm!(
                "ldp q0, q1, [{0}]",
                "ldp q2, q3, [{0}, #32]",
                // Continue for all registers you want to restore
                in(reg) self.0.as_ptr(),
                options(nostack, preserves_flags)
            );
        }
    }

    pub fn validate_at_resume(&self) -> HvResult {
        // Validation is much simpler on ARM; often you can just ensure data is non-zero or meets your app-specific checks
        if self.0.iter().all(|&x| x == 0) {
            return hv_result_err!(EINVAL, "FP/SIMD state is unexpectedly all zeros");
        }
        Ok(())
    }
}

impl Debug for FpSimdState {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_tuple("FpSimdState")
            .field(&self.0)
            .finish()
    }
}