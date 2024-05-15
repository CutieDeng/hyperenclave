use bitflags::bitflags;

#[repr(u32)]
#[derive(Debug)]
#[allow(dead_code)]
pub(super) enum ArmSysReg {
    MIDR_EL1 = 0x0, // Main ID Register
    ID_AA64PFR0_EL1 = 0x4, // ARMv8 Processor Feature Register 0
    ID_AA64DFR0_EL1 = 0x5, // Debug Feature Register 0
    ID_AA64ISAR0_EL1 = 0x6, // ISA Feature Register 0
    ID_AA64MMFR0_EL1 = 0x7, // Memory Model Feature Register 0
}

bitflags! {
    pub(super) struct ArmFeatureFlags: u64 {
        // ID_AA64PFR0_EL1 flags
        const EL3 = 1 << 0; // Secure EL3 available
        const EL2 = 1 << 1; // Hypervisor EL2 available
        const EL1 = 1 << 2; // Full-featured EL1
        const FP = 1 << 3; // Floating point support
        const ADVSIMD = 1 << 4; // Advanced SIMD support
        const GIC = 1 << 5; // Generic Interrupt Controller support
        const RAS = 1 << 6; // Reliability, Availability, and Serviceability extensions
        const SVE = 1 << 7; // Scalable Vector Extension support
        // Other features might include cryptographic extensions, etc., depending on your use case
    }
}

pub struct CpuFeatures {
    // For ARM, we typically gather features via ID registers instead of CPUID
    features: ArmFeatureFlags,
}

impl CpuFeatures {
    pub fn new() -> Self {
        // Simulate reading of system registers to determine features
        // This is a simplified placeholder to represent how you might initialize this struct on ARM
        let id_aa64pfr0_el1 = read_system_register(ArmSysReg::ID_AA64PFR0_EL1);
        Self {
            features: ArmFeatureFlags::from_bits_truncate(id_aa64pfr0_el1),
        }
    }

    pub fn has_virtualization(&self) -> bool {
        self.features.contains(ArmFeatureFlags::EL2)
    }

    pub fn has_floating_point(&self) -> bool {
        self.features.contains(ArmFeatureFlags::FP)
    }

    pub fn has_advsimd(&self) -> bool {
        self.features.contains(ArmFeatureFlags::ADVSIMD)
    }

    pub fn has_sve(&self) -> bool {
        self.features.contains(ArmFeatureFlags::SVE)
    }
    
    // Additional checks for other features can be similarly implemented
}

// Placeholder function to simulate reading from a system register
fn read_system_register(reg: ArmSysReg) -> u64 {
    match reg {
        ArmSysReg::ID_AA64PFR0_EL1 => {
            // Example: Let's assume the processor has EL2, FP, and Advanced SIMD
            (1 << 1) | (1 << 3) | (1 << 4) // EL2 + FP + ADVSIMD
        },
        _ => 0,
    }
}