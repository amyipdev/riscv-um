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
    // /// CPU type and instruction set (unimplemented)
    //#[arg(short, long)]
    //cpu: Option<String>,
    // /// Set maximum program memory allocation (unimplemented)
    //#[arg(short, long)]
    //mem: Option<usize>,
    filename: Option<String>,
    //#[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    //args: Vec<String>
}

fn main() {
    // Command line argument handling
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

    // Done this way for future development - cross-thread memory sharing
    let mut mem = mm::MemoryMap::new();
    let mut local_access = mem.clone();
    let mut mema = mem.lock().unwrap();

    // Load ELF data into memory; TODO make this faster, this memory system sucks
    // For POC, we assume .text+0x0 is the start instead of entry+0x0 or _start+0x0
    // a real emulator would need more complex logic here
    let sh = elf_f
        .section_header_by_name(".text")
        .e("ELF parsing error")
        .ok_or(elf::parse::ParseError::UnsupportedElfClass(0))
        .e("No .text section in the executable");
    let data = elf_f
        .section_data(&sh)
        .e("Failed to get .text section data");
    if data.1.is_some() {
        terminal_error("ELF compression is not supported in this proof-of-concept");
    }
    let entry_address = sh.sh_addr;
    let text_size = sh.sh_size;
    // Allocate necessary memory
    if !mema.allocate_address_range(entry_address, text_size) {
        terminal_error("Failed to allocate program space");
    }
    // Load program binary!!
    for (i, b) in data.0.into_iter().enumerate() {
        mema.writebyte(entry_address + (i as u64), *b);
    }

    // allocate 2 MiB stack
    let stack_size = 1 << 20;
    mema.allocate_address_range((1 << 39) - stack_size, stack_size);

    // registers initialization
    let mut registers: [u64; 32] = [0u64; 32];
    registers[2] = 1 << 39;
    let mut pc: u64 = entry_address;

    // command-line arguments
    // TODO in a future version, not sure if this is even along spec
    /*
    registers[10] = args.args.len(); // argc
    let argv_len = registers[10] << 3;
    registers[11] -= argv_len; // argv
    registers[2] -= argv_len;
    for (i, arg) in args.args.reverse().into_iter().enumerate() {
        let bytes = arg.bytes();
        let bl = bytes.len();
        registers[2] -= bl + 2;
        for n in 0..bl+1 {
            mema.writebyte(registers[2] + n, bytes[n]);
        }
        mema.writedword((1 << 39) - (i << 3), registers[2]);
    }*/

    //println!("{:?}", args.args);

    // Main CPU loop
    // TODO: factor opcode table out into separate file
    let opcode_table: [Box<dyn Fn(u32, &mut mm::MemoryMap, &mut [u64; 32], &mut u64) -> ()>; 128] = [
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|isn, _, registers, pc| {
            // opcode = 0010011 - R-type (OP-IMM)
            let imm = ((isn & 0xfff00000) >> 20) as u64;
            let src = ((isn & 0x000f8000) >> 15) as usize;
            let fct = (isn & 0x00007000) >> 12;
            let dst = ((isn & 0x00000f80) >> 7) as usize;
            let prefetch = registers[src];
            match fct {
                0x0000 => {
                    println!("addi");
                    utils::write_register_safe(registers, dst, prefetch + utils::sign_extend_12(imm));
                },
                0x1000 => {},
                0x2000 => {},
                0x3000 => {},
                0x4000 => {},
                0x5000 => {},
                0x6000 => {},
                0x7000 => {},
                _ => unimplemented!()
            }
            *pc += 4;
        }),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|isn, _, registers, pc| {
            // opcode = 1110011 - R-type (SYSTEM)
            let imm = ((isn & 0xfff00000) >> 20) as u64;
            let fct = (isn & 0x00007000) >> 12;
            if fct == 0 {
                if imm == 0 {
                    // ECALL
                    // TODO: probably should separate this into its own file
                    // TODO: use a lookup table instead of match
                    match registers[17] {
                        93 => std::process::exit(registers[10] as i32),
                        _ => unimplemented!()
                    }
                } else if imm == 1 {
                    // EBREAK
                    unimplemented!()
                } else {
                    unimplemented!()
                }
            }
            let src = ((isn & 0x000f8000) >> 15) as usize;
            let dst = ((isn & 0x00000f80) >> 7) as usize;
            match fct {
                _ => unimplemented!()
            }
        }),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
    ];
    // TODO split into threads for multiprocessing
    loop {
        // No compressed instruction support
        let isn = match mema.readword(pc) {
            Some(v) => v,
            None => terminal_error("sigsegv")
        };
        opcode_table[isn as usize & 0x7f](isn, &mut mema, &mut registers, &mut pc);
    }
}
