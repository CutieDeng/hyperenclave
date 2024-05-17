use bitflags::bitflags;

bitflags! {
    /// Memory access rights, similar in spirit to segment access rights in x86.
    pub struct MemoryAccessRights: u32 {
        const READABLE = 1 << 0;
        const WRITABLE = 1 << 1;
        const EXECUTABLE = 1 << 2;
        const PRESENT = 1 << 3;
    }
}

#[derive(Debug)]
pub struct MemoryRegion {
    pub start: u64,
    pub size: u64,
    pub access_rights: MemoryAccessRights,
}

impl MemoryRegion {
    pub const fn invalid() -> Self {
        Self {
            start: 0,
            size: 0,
            access_rights: MemoryAccessRights::empty(),
        }
    }

    pub fn new(start: u64, size: u64, rights: MemoryAccessRights) -> Self {
        Self {
            start,
            size,
            access_rights: rights,
        }
    }
}
