use crate::memory::PagingResult;
use crate::memory::{GenericPTE, MemFlags, PageTableLevel, PagingInstr, PhysAddr, VirtAddr};
use crate::memory::{Level4PageTable, Level4PageTableImmut, Level4PageTableUnlocked};


use core::fmt::{Debug, Formatter, Result};

// Replace x86_64 with the appropriate AArch64 abstractions or direct system calls.
use aarch64::{
    // Define or import structures and constants similar to AArch64 MMU operations.
};


// AArch64PageTableFlags 的定义，这里使用位掩码的方式定义各种标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AArch64PageTableFlags {
    pub bits: u64,
}

// MemFlags 到 AArch64PageTableFlags 的转换
impl From<MemFlags> for AArch64PageTableFlags {
    fn from(f: MemFlags) -> Self {
        let mut bits = 0;

        if !f.contains(MemFlags::NO_PRESENT) {
            bits |= Self::VALID;
        }
        if f.contains(MemFlags::WRITE) {
            bits |= Self::AP_RW;
        } else {
            bits |= Self::AP_RO;
        }
        if !f.contains(MemFlags::EXECUTE) {
            bits |= Self::UXN | Self::PXN;
        }
        if f.contains(MemFlags::ACCESSED) {
            bits |= Self::AF;
        }

        Self { bits }
    }
}

// 定义常用的页表标志
impl AArch64PageTableFlags {
    pub const VALID: u64 = 1 << 0;
    pub const TABLE: u64 = 1 << 1;
    pub const PAGE: u64 = 1 << 2;
    pub const AF: u64 = 1 << 10;
    pub const AP_RW: u64 = 1 << 7;
    pub const AP_RO: u64 = 1 << 6;
    pub const UXN: u64 = 1 << 54;
    pub const PXN: u64 = 1 << 53;
}


// AArch64PageTableFlags 到 MemFlags 的转换
impl From<AArch64PageTableFlags> for MemFlags {
    fn from(f: AArch64PageTableFlags) -> Self {
        let mut mem_flags = MemFlags::empty();

        if f.bits & AArch64PageTableFlags::VALID != 0 {
            mem_flags.remove(MemFlags::NO_PRESENT);
        } else {
            mem_flags.insert(MemFlags::NO_PRESENT);
        }
        if f.bits & AArch64PageTableFlags::AP_RW != 0 {
            mem_flags.insert(MemFlags::WRITE);
        }
        if f.bits & (AArch64PageTableFlags::UXN | AArch64PageTableFlags::PXN) != 0 {
            mem_flags.remove(MemFlags::EXECUTE);
        } else {
            mem_flags.insert(MemFlags::EXECUTE);
        }
        if f.bits & AArch64PageTableFlags::AF != 0 {
            mem_flags.insert(MemFlags::ACCESSED);
        }

        mem_flags
    }
}

// Physical address mask for AArch64, masking out the flags bits
const PHYS_ADDR_MASK: u64 = 0x0000_ffff_ffff_f000; // Commonly used for AArch64

#[derive(Clone)]
pub struct PTEntry(u64);

impl GenericPTE for PTEntry {
    // Extracts the physical address from the page table entry.
    fn addr(&self) -> PhysAddr {
        (self.0 & PHYS_ADDR_MASK) as _
    }
    
    // Converts the raw entry to memory flags.
    fn flags(&self) -> MemFlags {
        let bits = self.0 & !PHYS_ADDR_MASK;
        let mut mem_flags = MemFlags::empty();
        // if bits & (1 << 0) != 0 { mem_flags |= MemFlags::PRESENT; }
        if bits & (1 << 0) == 0 { mem_flags |= MemFlags::NO_PRESENT; }
        if bits & (1 << 1) != 0 { mem_flags |= MemFlags::WRITE; }
        if bits & (1 << 6) == 0 { mem_flags |= MemFlags::EXECUTE; } // No-Execute is inverted
        if bits & (1 << 2) != 0 { mem_flags |= MemFlags::USER; }
        mem_flags
    }
    
    // Checks if the entry is unused (all zeros).
    fn is_unused(&self) -> bool {
        self.0 == 0
    }
    
    // Checks if the entry is marked as present.
    fn is_present(&self) -> bool {
        self.0 & (1 << 0) != 0
    }
    
    // Determines if the entry is a leaf entry in the page table.
    fn is_leaf(&self) -> bool {
        // In AArch64, leaf can be identified by no further table pointers, which is specific to how it's used.
        // Here, we assume non-table (terminal) entries are leaves by specific flag patterns.
        (self.0 & (1 << 7)) != 0 // Example: might check for a specific 'large page' bit.
    }
    
    // Checks if the entry was recently accessed.
    fn is_young(&self) -> bool {
        self.0 & (1 << 5) != 0
    }
    
    // Marks the entry as not recently accessed.
    fn set_old(&mut self) {
        self.0 &= !(1 << 5);
    }
    
    // Sets the physical address in the entry.
    fn set_addr(&mut self, paddr: PhysAddr) {
        self.0 = (self.0 & !PHYS_ADDR_MASK) | (paddr as u64 & PHYS_ADDR_MASK);
    }
    
    // Sets the flags for the entry.
    fn set_flags(&mut self, flags: MemFlags, is_huge: bool) -> PagingResult {
        let mut bits = 0;
        // if flags.contains(MemFlags::PRESENT) { bits |= 1 << 0; }
        if !flags.contains(MemFlags::NO_PRESENT) { bits |= 1 << 0; } 
        if flags.contains(MemFlags::WRITE) { bits |= 1 << 1; }
        if !flags.contains(MemFlags::EXECUTE) { bits |= 1 << 6; }
        if flags.contains(MemFlags::USER) { bits |= 1 << 2; }
        if is_huge { bits |= 1 << 7; } // Setting a hypothetical 'large page' bit
        self.0 = (self.0 & PHYS_ADDR_MASK) | bits;
        Ok(())
    }
    
    // Sets the page table link in the entry.
    fn set_table(
        &mut self,
        paddr: PhysAddr,
        _next_level: PageTableLevel,
        is_present: bool,
    ) -> PagingResult {
        let mut bits = (1 << 1) | (1 << 2); // Writable and User-accessible
        if is_present { bits |= 1 << 0; }
        self.0 = (paddr as u64 & PHYS_ADDR_MASK) | bits;
        Ok(())
    }
    
    // Marks the entry as present.
    fn set_present(&mut self) -> PagingResult {
        self.0 |= 1 << 0;
        Ok(())
    }
    
    // Marks the entry as not present.
    fn set_notpresent(&mut self) -> PagingResult {
        self.0 &= !(1 << 0);
        Ok(())
    }
    
    // Clears the entry.
    fn clear(&mut self) {
        self.0 = 0
    }
}

impl Debug for PTEntry {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut f = f.debug_struct("PTEntry");
        f.field("raw", &self.0);
        f.field("addr", &self.addr());
        f.field("flags", &self.flags());
        f.finish()
    }
}

pub struct AArch64PagingInstr;

impl PagingInstr for AArch64PagingInstr {
    unsafe fn activate(root_paddr: PhysAddr) {
        // Set the TTBR0_EL1 or TTBR1_EL1 to activate the page tables.
    }

    fn flush(vaddr: Option<usize>) {
        // Use the appropriate TLB flush instructions for AArch64.
    }
}

pub type PageTable = Level4PageTable<VirtAddr, PTEntry, AArch64PagingInstr>;
pub type EnclaveGuestPageTableUnlocked = Level4PageTableUnlocked<VirtAddr, PTEntry, AArch64PagingInstr>;
pub type PageTableImmut = Level4PageTableImmut<VirtAddr, PTEntry>;
