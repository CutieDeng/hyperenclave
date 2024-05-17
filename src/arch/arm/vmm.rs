#[derive(Clone, Copy, Debug)]
pub struct NPTEntry {
    entry: u64,
}

impl NPTEntry {
    pub const fn new() -> Self {
        Self { entry: 0 }
    }

    pub fn is_unused(&self) -> bool {
        self.entry == 0
    }

    pub fn set_addr(&mut self, paddr: u64, flags: u64) {
        self.entry = (paddr & PHYS_ADDR_MASK) | (flags & !PHYS_ADDR_MASK);
    }

    pub fn clear(&mut self) {
        self.entry = 0;
    }
}

// 定义常见的页表项标志位
const PHYS_ADDR_MASK: u64 = 0x0000_ffff_ffff_f000;
const FLAG_PRESENT: u64 = 1 << 0;
const FLAG_WRITE: u64 = 1 << 1;
const FLAG_EXECUTE: u64 = 1 << 2;

use alloc::vec::Vec;

use crate::memory::{GenericPageTable, GenericPageTableImmut};

use super::GuestRegisters; 

pub struct NestedPageTable {
    // 嵌套页表通常使用多级页表结构，在这里我们假设只用一个简单数组模拟
    entries: Vec<NPTEntry>,
}

impl GenericPageTableImmut for NestedPageTable {
    type VA = usize;

    unsafe fn from_root(root_paddr: crate::memory::PhysAddr) -> Self {
        // todo!()
        Self::new(0) 
    }

    fn root_paddr(&self) -> crate::memory::PhysAddr {
        self.entries.first().map_or(0, |e| e.entry) 
    }

    fn query(&self, vaddr: Self::VA) -> crate::memory::PagingResult<(crate::memory::PhysAddr, crate::memory::MemFlags, crate::memory::PageSize)> {
        todo!()
    }
}

impl GenericPageTable for NestedPageTable {
    fn new() -> Self {
        NestedPageTable::new(0) 
    }

    fn map(&mut self, region: &crate::memory::MemoryRegion<Self::VA>) -> crate::memory::PagingResult {
        todo!()
    }

    fn unmap(&mut self, region: &crate::memory::MemoryRegion<Self::VA>)
        -> crate::memory::PagingResult<Vec<(crate::memory::PhysAddr, crate::memory::PageSize)>> {
        todo!()
    }

    fn update(&mut self, region: &crate::memory::MemoryRegion<Self::VA>) -> crate::memory::PagingResult {
        todo!()
    }

    fn clone(&self) -> Self {
        todo!()
    }

    unsafe fn activate(&self) {
        todo!()
    }

    fn flush(&self, vaddr: Option<Self::VA>) {
        todo!()
    }
}

impl NestedPageTable {
    pub fn new(size: usize) -> Self {
        Self {
            entries: vec![NPTEntry::new(); size],
        }
    }

    pub fn map(&mut self, vaddr: usize, paddr: u64, flags: u64) {
        // 模拟映射虚拟地址到物理地址，这里简化为直接映射索引
        let index = vaddr / 4096; // 假设每个页大小为 4KiB
        if index < self.entries.len() {
            self.entries[index].set_addr(paddr, flags);
        }
    }

    pub fn unmap(&mut self, vaddr: usize) {
        let index = vaddr / 4096;
        if index < self.entries.len() {
            self.entries[index].clear();
        }
    }

    pub fn query(&self, vaddr: usize) -> Option<u64> {
        let index = vaddr / 4096;
        self.entries.get(index).map(|e| e.entry)
    }
}

pub type EnclaveNestedPageTableUnlocked = NestedPageTable;


pub trait VcpuAccessGuestState {
    // Architecture independent methods:
    fn regs(&self) -> &GuestRegisters;
    fn regs_mut(&mut self) -> &mut GuestRegisters;
    fn instr_pointer(&self) -> u64;
    fn stack_pointer(&self) -> u64;
    fn frame_pointer(&self) -> u64; 
    fn set_stack_pointer(&mut self, sp: u64);
    fn set_return_val(&mut self, ret_val: usize); 

    // Methods only available for x86 cpus:
    fn rflags(&self) -> u64;
    fn fs_base(&self) -> u64;
    fn gs_base(&self) -> u64;
    fn efer(&self) -> u64;
    fn cr(&self, cr_idx: usize) -> u64;
    fn set_cr(&mut self, cr_idx: usize, val: u64);
    fn xcr0(&self) -> u64; 
    fn set_xcr0(&mut self, val: u64); 
}

pub struct AArch64Vcpu {
    pub registers: GuestRegisters, 
}

impl VcpuAccessGuestState for AArch64Vcpu {
    fn regs(&self) -> &GuestRegisters {
        &self.registers
    }

    fn regs_mut(&mut self) -> &mut GuestRegisters {
        &mut self.registers
    }

    fn instr_pointer(&self) -> u64 {
        self.registers.pc
    }

    fn stack_pointer(&self) -> u64 {
        self.registers.sp
    }

    fn frame_pointer(&self) -> u64 {
        self.registers.x[29]  // 使用 x29 作为帧指针
    }

    fn set_stack_pointer(&mut self, sp: u64) {
        self.registers.sp = sp;
    }

    fn set_return_val(&mut self, ret_val: usize) {
        self.registers.x[0] = ret_val as u64;  // 在 AArch64 上返回值使用 x0 寄存器
    }

    // 下面的方法在 AArch64 上没有直接对应，因此提供空实现或者适当的模拟
    fn rflags(&self) -> u64 {
        // self.registers.pstate  // 使用 pstate 作为类似 rflags 的状态寄存器
        0 
    }

    fn fs_base(&self) -> u64 {
        0  // AArch64 上没有直接对应的 fs_base
    }

    fn gs_base(&self) -> u64 {
        0  // AArch64 上没有直接对应的 gs_base
    }

    fn efer(&self) -> u64 {
        0  // AArch64 上没有直接对应的 EFER 寄存器
    }

    fn cr(&self, _cr_idx: usize) -> u64 {
        0  // AArch64 上没有类似 x86 的控制寄存器 CR
    }

    fn set_cr(&mut self, _cr_idx: usize, _val: u64) {
        // AArch64 上没有类似 x86 的控制寄存器 CR
    }

    fn xcr0(&self) -> u64 {
        0  // AArch64 上没有 XCR0 寄存器
    }

    fn set_xcr0(&mut self, _val: u64) {
        // AArch64 上没有 XCR0 寄存器
    }
} 

