// Modified by Ant Group in 2023.

// Copyright (C) 2023 Ant Group CO., Ltd. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use core::cmp;
use core::fmt;
use core::mem;

use core::arch::global_asm;
use core::mem::MaybeUninit; 

// 当编译目标不是 x86 / x86-64 时，该操作不成立
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))] 
global_asm!(include_str!("rand.S"), options(att_syntax));

#[cfg(any(target_arch = "aarch64"))] 
global_asm!(include_str!("rand-arm.S")); 

#[inline]
fn getrandom(buf: &mut [u8]) {
    extern "C" {
        // fn do_rdrand(rand: *mut u32) -> u32;
        // Actually, it's more proper when using this method 
        fn do_rdrand(rand_result: &mut MaybeUninit<u32>) -> u32; 
    }

    let mut rand_num : MaybeUninit<u32> = MaybeUninit::uninit(); 
    let mut to_fill = &mut buf[..]; 

    while !to_fill.is_empty() {
        // 一旦访问随机数失败，就触发一个非法指令操作... 
        // 换言之，即断言该操作一定成功
        if unsafe { do_rdrand(&mut rand_num) } == 0 {
            core::intrinsics::abort()
        }

        let copy_len = cmp::min(mem::size_of_val(to_fill), mem::size_of_val(&rand_num)); 
        to_fill[..copy_len].copy_from_slice( unsafe { &rand_num.assume_init_ref().to_ne_bytes() } ); 
        to_fill = &mut to_fill[copy_len..]; 
    }
}

fn next_u32(fill_buf: &mut dyn FnMut(&mut [u8])) -> u32 {
    let mut buf: [u8; 4] = [0; 4];
    fill_buf(&mut buf);
    unsafe { mem::transmute::<[u8; 4], u32>(buf) }
}

fn next_u64(fill_buf: &mut dyn FnMut(&mut [u8])) -> u64 {
    let mut buf: [u8; 8] = [0; 8];
    fill_buf(&mut buf);
    unsafe { mem::transmute::<[u8; 8], u64>(buf) }
}

fn next_usize(fill_buf: &mut dyn FnMut(&mut [u8])) -> usize {
    let mut buf: [u8; mem::size_of::<usize>()] = [0; mem::size_of::<usize>()];
    fill_buf(&mut buf);
    unsafe { mem::transmute::<[u8; mem::size_of::<usize>()], usize>(buf) }
}

// A random number generator
pub struct Rng;

impl Rng {
    pub fn new() -> Rng {
        Rng
    }

    pub fn next_u32(&mut self) -> u32 {
        next_u32(&mut getrandom)
    }

    pub fn next_u64(&mut self) -> u64 {
        next_u64(&mut getrandom)
    }

    pub fn next_usize(&mut self) -> usize {
        next_usize(&mut getrandom)
    }

    pub fn fill_bytes(&mut self, buf: &mut [u8]) {
        getrandom(buf)
    }
}

impl fmt::Debug for Rng {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rng {{}}")
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self::new()
    }
}
