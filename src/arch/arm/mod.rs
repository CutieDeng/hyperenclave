mod enclave;  // Secure enclave implementation
mod stage2_page_table;  // Stage 2 page table handling
mod vcpu;  // Virtual CPU state and operations
mod exception;  // Exception handling for secure and non-secure states
mod context; 

use crate::error::{HvError, HvResult};
use crate::arch::cpu::check_cpu_features;

// Simplified modules for clarity
pub use enclave::{EnclaveExceptionInfo, EnclaveThreadState};
pub use stage2_page_table::{S2PTEntry, Stage2PageTable};
pub use vcpu::Vcpu;
pub use exception::{ExceptionInfo, ExceptionType};

// Check virtualization and security features at the Hypervisor level
pub fn check_hypervisor_feature() -> HvResult {
    // Ensure the CPU supports the necessary virtualization features
    if !check_cpu_features().has_virtualization() {
        warn!("ARM Virtualization not supported!");
        return hv_result_err!(ENODEV, "Virtualization feature checks failed!");
    }

    // Validate hypervisor configuration settings
    let hcr_el2 = read_hcr_el2();
    if (hcr_el2 & HCR_EL2_MIN_REQUIRED) != HCR_EL2_MIN_REQUIRED {
        return hv_result_err!(ENODEV, "Required HCR_EL2 flags checks failed!");
    }

    Ok(())
}

fn read_hcr_el2() -> u64 {
    // Placeholder function to read the HCR_EL2 system register
    // In actual implementation, this would involve specific system calls or privileged instructions
    0x0000_0000_0000_0000  // Example value for placeholder
}