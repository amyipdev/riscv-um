// SPDX-License-Identifier: GPL-2.0-or-later

use colored::Colorize;

// Not enough error handling logic to justify its own file

pub(crate) fn terminal_error(msg: &str) -> ! {
    println!("{} {}", "error:".red().bold(), msg);
    std::process::exit(1);
}

pub(crate) trait ConvertibleError<T> {
    fn e(self, msg: &str) -> T;
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
            Err(_) => terminal_error(msg)
        }
    }
}
