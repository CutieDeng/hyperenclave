mod enclave;
mod s2pt; // Stage 2 Page Table, analogous to Intel's EPT
mod structs;
mod vcpu;
mod vmexit;
mod smmu; // System Memory Management Unit, analogous to Intel's VT-d

use libvmm::sysreg; // Assuming there's an equivalent system register handler in libvmm
use crate::arch::cpu::check_cpuid;
use crate::arch::cpuid::CpuFeatures;
use crate::error::{HvError, HvResult};

pub use s2pt::S2PTEntry as NPTEntry;
pub use s2pt::EnclaveStage2PageTableUnlocked as EnclaveNestedPageTableUnlocked;
pub use s2pt::Stage2PageTable as NestedPageTable;
pub use vcpu::Vcpu;
pub use smmu::{SmmuPTEntry, SmmuPageTable, Iommu};

const HCR_EL2_MIN_REQUIRED: u64 = /* Hypervisor Configuration flags needed for your VMs */

use core::convert::From; 

impl From<SysRegFail> for HvError {
    fn from(err: SysRegFail) -> Self {
        match err {
            SysRegFail::ReadError => hv_err!(EIO, format!("{:?}", err)),
            _ => hv_err!(EIO, format!("{:?}", err)),
        }
    }
}

pub fn check_hypervisor_feature() -> HvResult {
    // Check cpuid for ARM feature support
    check_cpuid()?;

    // Check if CPU features support virtualization
    if !CpuFeatures::new().has_virtualization() {
        warn!("ARM Virtualization not supported!");
        return hv_result_err!(ENODEV, "Virtualization feature checks failed!");
    }

    let hcr_el2 = sysreg::HCR_EL2.read();
    if (hcr_el2 & HCR_EL2_MIN_REQUIRED) != HCR_EL2_MIN_REQUIRED {
        return hv_result_err!(ENODEV, "required HCR_EL2 flags checks failed!");
    }

    Ok(())
}