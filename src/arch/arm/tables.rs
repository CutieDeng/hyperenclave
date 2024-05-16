use core::arch::asm;
use spin::Mutex;

#[cfg(feature = "arm")]
use core::sync::atomic::{AtomicU64, Ordering};

pub(super) static IVT: Mutex<IVTStruct> = Mutex::new(IVTStruct::new());

#[derive(Debug)]
pub struct IVTStruct {
    vector_table: [AtomicU64; 256],
}

impl IVTStruct {
    pub fn new() -> Self {
        Self {
            vector_table: Default::default(),
        }
    }

    pub fn load(&mut self) {
        let base = self.vector_table.as_ptr() as u64;
        unsafe { asm!("msr vbar_el1, {}", in(reg) base, options(nostack, preserves_flags)) };
    }

    pub fn set_handler(&mut self, index: usize, handler: extern "C" fn()) {
        self.vector_table[index].store(handler as u64, Ordering::SeqCst);
    }
}

extern "C" {
    #[link_name = "exception_entries"]
    static ENTRIES: [extern "C" fn(); 256];
}

pub(super) fn initialize_ivt() {
    let mut ivt = IVT.lock();
    for i in 0..256 {
        ivt.set_handler(i, unsafe { core::mem::transmute(ENTRIES[i]) });
    }
    ivt.load();
}