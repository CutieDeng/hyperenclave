use bit_field::BitField;
use bitflags::bitflags;
use aarch64_cpu::registers::{SCTLR_EL1, TCR_EL1};
// use cortex_a::asm;

bitflags! {
    /// Access rights for VMCS guest register states (模拟x86-64的段访问权限)
    pub struct SegmentAccessRights: u32 {
        /// Accessed flag
        const ACCESSED          = 1 << 0;
        /// Executable flag (设置为1表示可执行)
        const EXECUTABLE        = 1 << 1;
        /// Writable flag (设置为1表示可写)
        const WRITABLE          = 1 << 2;
        /// Cacheable flag (设置为1表示可缓存)
        const CACHEABLE         = 1 << 3;
        /// Bufferable flag (设置为1表示可缓冲)
        const BUFFERABLE        = 1 << 4;
        /// Present flag (设置为1表示有效)
        const PRESENT           = 1 << 7;
    }
}

impl SegmentAccessRights {
    #[allow(dead_code)]
    pub fn from_descriptor(desc: u64) -> Self {
        Self::from_bits_truncate(desc.get_bits(40..56) as u32)
    }
}

#[derive(Debug)]
pub struct Segment {
    pub base: u64,
    pub limit: u64,
    pub access_rights: SegmentAccessRights,
}

impl Segment {
    pub const fn invalid() -> Self {
        Self {
            base: 0,
            limit: 0,
            access_rights: SegmentAccessRights::empty(),
        }
    }

    pub fn from_descriptor(base: u64, limit: u64, rights: u32) -> Self {
        Self {
            base,
            limit,
            access_rights: SegmentAccessRights::from_bits_truncate(rights),
        }
    }
}