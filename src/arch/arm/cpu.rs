use crate::error::HvResult;

pub fn core_id() -> usize {
    // On many ARM systems, you can use MPIDR_EL1 to find core information
    let core_id: u64;
    unsafe { core::arch::asm!("mrs {core_id}, mpidr_el1", core_id = out(reg) core_id) };
    (core_id & 0xff) as usize  // Just an example to extract core ID
}

pub fn time_now() -> u64 {
    let cntvct: u64;
    unsafe { core::arch::asm!("mrs {cntvct}, cntvct_el0", cntvct = out(reg) cntvct) };
    cntvct
}

pub fn check_cpu_features() -> HvResult {
    // Example for checking NEON support
    let has_neon: bool;
    unsafe { core::arch::asm!("mrs {has_neon}, id_aa64isar0_el1", has_neon = out(reg) has_neon) };
    if has_neon & (1 << 20) == 0 {
        return hv_result_err!(ENODEV, "NEON is not supported!");
    }
    Ok(())
}

#[allow(dead_code)]
const CACHE_LINE_SIZE: usize = 64;

#[allow(dead_code)]
pub fn clflush_cache_range(vaddr: usize, length: usize) {
    // ARM uses a different mechanism for cache operations
    for addr in (vaddr..(vaddr + length)).step_by(CACHE_LINE_SIZE) {
        unsafe {
            core::arch::asm!(
                "dc cvac, {0}",
                in(reg) addr,
                options(nostack)
            );
        }
    }
    // ARM uses DMB (Data Memory Barrier) for ordering
    unsafe { core::arch::asm!("dmb ish") };
}