// SPDX-License-Identifier: GPL-2.0-or-later

mod mm;
mod utils;

use clap::Parser;

use utils::ConvertibleError;
use utils::terminal_error;

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
    /// CPU type and instruction set (unimplemented)
    #[arg(short, long)]
    cpu: Option<String>,
    /// Set maximum program memory allocation (unimplemented)
    #[arg(short, long)]
    mem: Option<usize>,
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

    // Load ELF
    let path = std::path::PathBuf::from(&args.filename.unwrap());
    if !path.exists() {
        terminal_error("No such file or directory");
    }
    let fc = std::fs::read(&path).e("Error reading file");
    // check for ELF
    if fc.len() < 4 || !(fc[0] == 0x7f && fc[1] == b'E' && fc[2] == b'L' && fc[3] == b'F') {
        terminal_error("Non-ELF executables are currently not supported");
    }
    let elf_f = elf::ElfBytes::<elf::endian::LittleEndian>::minimal_parse(&fc)
        .e("Unable to parse ELF file");

    // Check that the file is RISC-V 64, Linux
    if elf_f.ehdr.class != elf::file::Class::ELF64 {
        terminal_error("32-bit ELF files are not supported");
    }
    if elf_f.ehdr.osabi != elf::abi::ELFOSABI_SYSV {
        terminal_error("File is not linked for Unix System V ABI");
    }
    if elf_f.ehdr.e_machine != elf::abi::EM_RISCV {
        terminal_error("File architecture is not RISC-V");
    }

    let mut mem = mm::MemoryMap::new();
    let mut local_access = mem.clone();
    let mut mema = mem.lock().unwrap();
    // Memory demo
    let addr = 0x8000;
    let byte = 0x41;
    mema.allocate_known_page(addr);
    mema.writebyte(addr, byte);
    println!("Reading {:#x}, loaded {:#x}", addr, mema.readbyte(addr).unwrap());

}
