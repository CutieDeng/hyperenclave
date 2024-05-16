use core::fmt::{Debug, Formatter, Result};
use core::arch::asm;

use crate::memory::{PagingResult, GenericPTE, MemFlags, PageTableLevel, PagingInstr, PhysAddr, VirtAddr};

bitflags! {
    pub struct PageTableFlags: u32 {
        const TYPE_MASK = 0b11;
        const TYPE_BLOCK = 0b01;
        const TYPE_TABLE = 0b11;
        const USER_ACCESSIBLE = 1 << 6;
        const READ_WRITE = 1 << 7;
        const EXECUTE_NEVER = 1 << 54;
    }
}

impl From<MemFlags> for PageTableFlags {
    fn from(f: MemFlags) -> Self {
        let mut ret = Self::empty();
        if !f.contains(MemFlags::NO_PRESENT) {
            ret |= Self::TYPE_TABLE;  // Treat as present
        }
        if f.contains(MemFlags::WRITE) {
            ret |= Self::READ_WRITE;
        }
        if !f.contains(MemFlags::EXECUTE) {
            ret |= Self::EXECUTE_NEVER;
        }
        if f.contains(MemFlags::USER) {
            ret |= Self::USER_ACCESSIBLE;
        }
        ret
    }
}

impl From<PageTableFlags> for MemFlags {
    fn from(f: PageTableFlags) -> Self {
        let mut ret = Self::READ;
        if f.contains(PageTableFlags::TYPE_TABLE) {
            ret &= !Self::NO_PRESENT;
        }
        if f.contains(PageTableFlags::READ_WRITE) {
            ret |= Self::WRITE;
        }
        if !f.contains(PageTableFlags::EXECUTE_NEVER) {
            ret |= Self::EXECUTE;
        }
        if f.contains(PageTableFlags::USER_ACCESSIBLE) {
            ret |= Self::USER;
        }
        ret
    }
}

const PHYS_ADDR_MASK: u64 = 0x0000_ffff_ffff_f000;

#[derive(Clone)]
pub struct PTEntry(u64);

impl GenericPTE for PTEntry {
    fn addr(&self) -> PhysAddr {
        (self.0 & PHYS_ADDR_MASK) as _
    }
    fn flags(&self) -> MemFlags {
        PageTableFlags::from_bits_truncate(self.0).into()
    }
    fn is_unused(&self) -> bool {
        self.0 == 0
    }
    fn is_present(&self) -> bool {
        PageTableFlags::from_bits_truncate(self.0).contains(PageTableFlags::TYPE_TABLE)
    }
    fn is_leaf(&self) -> bool {
        // In ARM, a leaf can be a block or a page
        self.0 & PageTableFlags::TYPE_MASK.bits() == PageTableFlags::TYPE_BLOCK.bits()
    }
    fn is_young(&self) -> bool {
        false  // ARM does not have a direct equivalent to x86 ACCESSED flag
    }
    fn set_old(&mut self) {
        // ARM does not have a direct equivalent to x86 ACCESSED flag
    }
    fn set_addr(&mut self, paddr: PhysAddr) {
        self.0 = (self.0 & !PHYS_ADDR_MASK) | (paddr as u64 & PHYS_ADDR_MASK);
    }
    fn set_flags(&mut self, flags: MemFlags, is_huge: bool) -> PagingResult {
        let mut pte_flags = PageTableFlags::from(flags);
        if is_huge {
            // Adapt huge page settings for ARM if necessary
            pte_flags |= PageTableFlags::TYPE_BLOCK;
        }
        self.0 = self.addr() as u64 | pte_flags.bits();
        Ok(())
    }
    fn set_table(
        &mut self,
        paddr: PhysAddr,
        _next_level: PageTableLevel,
        is_present: bool,
    ) -> PagingResult {
        let mut flags = PageTableFlags::READ_WRITE | PageTableFlags::USER_ACCESSIBLE;
        if is_present {
            flags |= PageTableFlags::TYPE_TABLE;
        }
        self.0 = (paddr as u64 & PHYS_ADDR_MASK) | flags.bits();
        Ok(())
    }
    fn set_present(&mut self) -> PagingResult {
        self.0 |= PageTableFlags::TYPE_TABLE.bits();
        Ok(())
    }
    fn set_notpresent(&mut self) -> PagingResult {
        let mut flags = PageTableFlags::from_bits_truncate(self.0);
        flags -= PageTableFlags::TYPE_TABLE;
        self.0 = self.addr() as u64 | flags.bits();
        Ok(())
    }
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

pub struct ARMPagingInstr;

impl PagingInstr for ARMPagingInstr {
    unsafe fn activate(root_paddr: PhysAddr) {
        // Set TTBR0 for user space or TTBR1 for kernel space based on context
        asm!(
            "msr ttbr0_el1, {0}",
            in(reg) root_paddr,
        );
    }

    fn flush(vaddr: Option<usize>) {
        if let Some(vaddr) = vaddr {
            // Flush specific virtual address from TLB
            asm!(
                "dsb ishst",
                "tlbi vae1is, {0}",
                "dsb ish",
                "isb",
                in(reg) vaddr >> 12,
            );
        } else {
            // Flush entire TLB
            asm!(
                "dsb ishst",
                "tlbi vmalle1is",
                "dsb ish",
                "isb"
            );
        }
    }
}

pub type PageTable = Level4PageTable<VirtAddr, PTEntry, ARMPagingInstr>;
pub type EnclaveGuestPageTableUnlocked = Level4PageTableUnlocked<VirtAddr, PTEntry, ARMPagingInstr>;
pub type PageTableImmut = Level4PageTableImmut<VirtAddr, PTEntry>;