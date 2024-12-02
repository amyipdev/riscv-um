// SPDX-License-Identifier: GPL-2.0-or-later

use arrayref::array_ref;
use colored::Colorize;

// Not enough error handling logic to justify its own file

pub(crate) fn terminal_error(msg: &str) -> ! {
    println!("{} {}", "error:".red().bold(), msg);
    std::process::exit(1);
}

pub(crate) trait ConvertibleError<T> {
    fn e(self, msg: &str) -> T;
}

#[inline(always)]
pub(crate) fn write_register_safe(reg: &mut [u64; 32], reg_num: usize, val: u64) -> () {
    if reg_num != 0 {
        reg[reg_num] = val;
    }
}

#[inline(always)]
pub(crate) fn sign_extend_12(val: u64) -> u64 {
    let b = (val >> 11) & 1;
    let m = b * !0u64 << 11;
    (val & 0xfff) | m
}

#[inline(always)]
pub(crate) fn sign_extend_13(val: u64) -> u64 {
    let b = (val >> 12) & 1;
    let m = b * !0u64 << 12;
    (val & 0x1fff) | m
}

#[inline(always)]
pub(crate) fn sign_extend_20(val: u64) -> u64 {
    let b = (val >> 19) & 1;
    let m = b * !0u64 << 19;
    (val & 0xfffff) | m
}

#[inline(always)]
pub(crate) fn sign_extend_32(val: u64) -> u64 {
    let b = (val >> 31) & 1;
    let m = b * !0u64 << 31;
    (val & 0xffff_ffff) | m
}

pub(crate) fn extract_u16_from_page(array: &[u8; 4096], start: usize) -> u16 {
    let bytes = array_ref![array, start, 2];
    u16::from_le_bytes(*bytes)
}

pub(crate) fn extract_u32_from_page(array: &[u8; 4096], start: usize) -> u32 {
    let bytes = array_ref![array, start, 4];
    u32::from_le_bytes(*bytes)
}

pub(crate) fn extract_u64_from_page(array: &[u8; 4096], start: usize) -> u64 {
    let bytes = array_ref![array, start, 8];
    u64::from_le_bytes(*bytes)
}

// Why does Box implement Error???
//impl<T, E: std::error::Error> ConvertibleError<T> for Result<T, Box<E>> {
//    fn e(self, msg: &str) -> T {
//        match self {
//            Ok(v) => v,
//            Err(_) => terminal_error(msg)
//        }
//    }
//}

impl<T, E: std::error::Error> ConvertibleError<T> for Result<T, E> {
    fn e(self, msg: &str) -> T {
        match self {
            Ok(v) => v,
            Err(_) => terminal_error(msg),
        }
    }
}
