// SPDX-License-Identifier: GPL-2.0-or-later

mod utils;

use clap::Parser;

use utils::terminal_error;
use utils::ConvertibleError;

static ABOUT_MSG: &'static str = "riscv-um: user mode RISC-V emulator
Copyright (c) 2024 Amy Parker <amy@amyip.net>

Repository: <https://github.com/amyipdev/riscv-um>

This program is free software; you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 2 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY: without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.";

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Display copyright and program information
    #[arg(short, long)]
    about: bool,
    /// CPU type and instruction set
    #[arg(short, long)]
    cpu: Option<String>,
    /// Set maximum program memory allocation
    #[arg(short, long)]
    mem: Option<usize>,
    /// Disable emulator memory paging
    #[arg(short = 'p', long)]
    nopaging: bool,
    filename: Option<String>,
}

fn main() {
    // Command line argument handling
    // TODO: profile how long this part takes
    let args = Args::parse();
    if args.about {
        println!("{}", ABOUT_MSG);
        std::process::exit(0);
    }
    if args.filename.is_none() {
        terminal_error("No executable specified");
    }

    let path = std::path::PathBuf::from(&args.filename.unwrap());
    if !path.exists() {
        terminal_error("No such file or directory");
    }
    let fc = std::fs::read(&path).e("Error reading file");
    // check for ELF
}
