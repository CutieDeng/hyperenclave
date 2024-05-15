use bitflags::bitflags;

#[repr(u64)]
#[derive(Debug)]
#[allow(dead_code)]
pub(super) enum ArmSysReg {
    ID_AA64PFR0_EL1 = 0x4, // Processor Feature Register 0
    ID_AA64ISAR0_EL1 = 0x6, // ISA Feature Register 0
    ID_AA64MMFR0_EL1 = 0x7, // Memory Model Feature Register 0
}

bitflags! {
    pub(super) struct ArmFeatureFlags: u64 {
        const FP = 1 << 0; // Floating point support
        const ADVSIMD = 1 << 1; // Advanced SIMD support
        const EL2 = 1 << 2; // Virtualization support
        const EL3 = 1 << 3; // Secure EL3 support
        const AES = 1 << 4; // AES instructions support
        const SHA1 = 1 << 5; // SHA1 instructions support
        const SHA256 = 1 << 6; // SHA256 instructions support
        const ATOMIC = 1 << 7; // Atomic instructions support
    }
}

pub struct CpuFeatures {
    features: ArmFeatureFlags,
}

impl CpuFeatures {
    pub fn new() -> Self {
        let pfr0 = read_system_register(ArmSysReg::ID_AA64PFR0_EL1);
        let isar0 = read_system_register(ArmSysReg::ID_AA64ISAR0_EL1);
        Self {
            features: ArmFeatureFlags::from_bits_truncate(pfr0 | isar0),
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

    pub fn has_aes(&self) -> bool {
        self.features.contains(ArmFeatureFlags::AES)
    }

    pub fn has_sha1(&self) -> bool {
        self.features.contains(ArmFeatureFlags::SHA1)
    }

    pub fn has_sha256(&self) -> bool {
        self.features.contains(ArmFeatureFlags::SHA256)
    }

    pub fn has_atomic(&self) -> bool {
        self.features.contains(ArmFeatureFlags::ATOMIC)
    }
}

fn read_system_register(reg: ArmSysReg) -> u64 {
    let value: u64;
    unsafe {
        match reg {
            ArmSysReg::ID_AA64PFR0_EL1 => {
                core::arch::asm!("mrs {value}, ID_AA64PFR0_EL1", value = out(reg) value);
            }
            ArmSysReg::ID_AA64ISAR0_EL1 => {
                core::arch::asm!("mrs {value}, ID_AA64ISAR0_EL1", value = out(reg) value);
            }
            ArmSysReg::ID_AA64MMFR0_EL1 => {
                core::arch::asm!("mrs {value}, ID_AA64MMFR0_EL1", value = out(reg) value);
            }
            _ => {
                value = 0;
            }
        }
    }
    value
}