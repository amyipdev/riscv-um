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
