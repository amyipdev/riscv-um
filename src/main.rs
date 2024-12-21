// SPDX-License-Identifier: GPL-2.0-or-later

#![feature(vec_into_raw_parts)]

#[cfg(not(target_endian = "little"))]
compile_error!("Host architecture must be little endian");

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

const stack_size: u64 = 1 << 20;
const stack_bottom: u64 = (1 << 39) - stack_size;

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
    //let mut mem = mm::MemoryMap::new();
    //let mut local_access = mem.clone();
    //let mut mema = mem.lock().unwrap();
    let mema: *mut libc::c_void = unsafe {libc::mmap(0 as *mut libc::c_void, 1 << 24, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_PRIVATE | libc::MAP_ANONYMOUS, -1, 0)};
    unsafe { libc::madvise(mema, 1 << 24, libc::MADV_HUGEPAGE); }

    // Load ELF data into memory; TODO make this faster, this memory system sucks
    // a real emulator would need more complex logic here
    let mut shs = elf_f.section_headers();
    if shs.is_none() {
        terminal_error("No ELF sections");
    }
    for shr in shs.unwrap().iter() {
        let data = elf_f.section_data(&shr).e("Failed to get section data");
        if data.1.is_some() {
            terminal_error("ELF compression is not yet supported");
        }
        // Incomplete: other sections may need to be loaded and read
        if shr.sh_type != 0x1 {
            continue;
        }
        let entry_address = shr.sh_addr;
        let text_size = shr.sh_size;
        /*
        if !mema.allocate_address_range(entry_address, text_size) {
            terminal_error("Failed to allocate program space");
        }*/
        unsafe { libc::memcpy(mema.byte_offset(entry_address as isize) as *mut libc::c_void, data.0.as_ptr() as *const libc::c_void, text_size as libc::size_t); }
        //for (i, b) in data.0.into_iter().enumerate() {
        //    mema.writebyte(entry_address + (i as u64), *b);
        //}
    }
    /*
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
    }*/
    let entry_address = 'L0: {
        let data = elf_f
            .symbol_table()
            .e("Failed to get symbol table")
            .ok_or(elf::parse::ParseError::UnexpectedSectionType((0, 0)))
            .e("Symbol table is empty");
        for sym in data.0 {
            if let Ok(v) = data.1.get(sym.st_name as usize) {
                if v == "_start" {
                    break 'L0 sym.st_value;
                }
            }
        }
        terminal_error("Could not find _start")
    };

    // allocate 2 MiB stack
    //mema.allocate_address_range((1 << 39) - stack_size, stack_size);

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
    let opcode_table: [Box<dyn Fn(u32, *mut libc::c_void, &mut [u64; 32], &mut u64) -> ()>; 128] = [
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|isn, mem, registers, pc| {
            let fct = (isn & 0x00007000) >> 12;
            let rd = ((isn & 0x0000_0f80) >> 7) as usize;
            let rs = ((isn & 0x000f_8000) >> 15) as usize;
            let imm = ((isn & 0xfff0_0000) >> 20) as u64;
            let addr = registers[rs] + utils::sign_extend_12(imm);
            match fct {
                2 => {
                        //println!("lw %{},0x${:x?} = {}", rd, addr, r);
                        utils::write_register_safe(
                            registers,
                            rd,
                            utils::sign_extend_32(unsafe {*(adt(addr, mem) as *const u32) as u64})
                        );
                }
                3 => {
                        //println!("ld %{},0x${:x?} = {}", rd, addr, r);
                    utils::write_register_safe(registers, rd, unsafe {*(adt(addr, mem) as *const u64)});
                }
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
        Box::new(|isn, _, registers, pc| {
            // opcode = 0010011 - R-type (OP-IMM)
            let imm = ((isn & 0xfff00000) >> 20) as u64;
            let src = ((isn & 0x000f8000) >> 15) as usize;
            let fct = (isn & 0x00007000) >> 12;
            let dst = ((isn & 0x00000f80) >> 7) as usize;
            let prefetch = registers[src];
            match fct {
                0 => {
                    //println!("addi d%{},%{},0x{:x?} = 0x{:x?}", dst, src, utils::sign_extend_12(imm) as i64, prefetch + utils::sign_extend_12(imm));
                    utils::write_register_safe(
                        registers,
                        dst,
                        prefetch + utils::sign_extend_12(imm),
                    );
                }
                1 => {
                    let shamt = imm & 0x3f;
                    //println!("slli d%{},%{},0x{:x?} = 0x{:x?}", dst, src, shamt, prefetch << shamt);
                    match imm & 0xfc0 {
                        0 => utils::write_register_safe(
                            registers,
                            dst,
                            prefetch << shamt
                        ),
                        _ => unimplemented!()
                    }
                },
                2 => unimplemented!(),
                3 => unimplemented!(),
                4 => unimplemented!(),
                5 => {
                    let shamt = imm & 0x3f;
                    //println!("srli d%{},%{},0x{:x?} = 0x{:x?}", dst, src, shamt, prefetch >> shamt);
                    match imm & 0xfc0 {
                        0 => utils::write_register_safe(
                            registers,
                            dst,
                            prefetch >> shamt
                        ),
                        _ => unimplemented!()
                    }
                },
                6 => unimplemented!(),
                7 => {
                    let se = utils::sign_extend_12(imm);
                    //println!("andi d%{},%{},0x{:x?} = 0x{:x?}", dst, src, se as i64, prefetch & se);
                    utils::write_register_safe(
                        registers,
                        dst,
                        prefetch & se
                    );
                },
                _ => unimplemented!(),
            }
            *pc += 4;
        }),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|isn, _, registers, pc| {
            let rd = ((isn & 0x00000f80) >> 7) as usize;
            let val = utils::sign_extend_32((isn & 0xfffff000) as u64);
            //println!("auipc pc=0x{:x?},val={}", *pc, (val as i64));
            utils::write_register_safe(
                registers,
                rd,
                *pc + val
            );
            *pc += 4;
        }),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|isn, _, registers, pc| {
            let imm = ((isn & 0xfff00000) >> 20) as u64;
            let src = ((isn & 0x000f8000) >> 15) as usize;
            let fct = (isn & 0x00007000) >> 12;
            let dst = ((isn & 0x00000f80) >> 7) as usize;
            let prefetch = registers[src] as u32;
            match fct {
                0 => {
                    //println!("addiw d%{},%{},0x{:x?} = 0x{:x?}", dst, src, imm, utils::sign_extend_32((prefetch + (imm as u32)) as u64));
                    utils::write_register_safe(
                        registers,
                        dst,
                        utils::sign_extend_32((prefetch + (utils::sign_extend_12(imm) as u32)) as u64),
                    );
                }
                1 => {
                    let shamt = imm & 0x1f;
                    //println!("slliw d%{},%{},0x{:x?} = 0x{:x?}", dst, src, imm, ((prefetch << shamt) as u64));
                    match imm & 0xfe0 {
                        0 => utils::write_register_safe(
                            registers,
                            dst,
                            (prefetch << shamt) as u64
                        ),
                        _ => unimplemented!()
                    }
                },
                2 => unimplemented!(),
                3 => unimplemented!(),
                4 => unimplemented!(),
                5 => unimplemented!(),
                6 => unimplemented!(),
                7 => unimplemented!(),
                _ => unimplemented!(),
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
        Box::new(|isn, mem, registers, pc| {
            // opcode = 0100011 - S-type
            let imm = ((((isn & 0xfe000000) >> 20) as u64) | (((isn & 0x00000f80) >> 7) as u64));
            let pf = registers[((isn & 0x000f8000) >> 15) as usize];
            let dst = imm + pf;
            let src = ((isn & 0x01f00000) >> 20) as usize;
            let fct = (isn & 0x00007000) >> 12;
            match fct {
                0 => {
                    //println!("sb ${:x?},%{} = 0x{:x?}", dst, src, registers[src] as u8);
                    unsafe {*(adt(dst, mem) as *mut u8) = registers[src] as u8};
                    //if !mem.writebyte(dst, registers[src] as u8) {
                    //    unsafe {
                    //        libc::raise(11);
                    //    }
                    //}
                },
                1 => unimplemented!(),
                2 => {
                    //println!("sw ${:x?},%{} = 0x{:x?}", dst, src, registers[src] as u32);
                    unsafe {*(adt(dst, mem) as *mut u32) = registers[src] as u32};
                    /*for (i, n) in (registers[src] as u32).to_le_bytes().into_iter().enumerate() {
                        if !mem.writebyte(dst + (i as u64), n) {
                            unsafe {
                                libc::raise(11);
                            }
                        }
                    }*/
                },
                3 => {
                    //println!("sd ${:x?},%{} = 0x{:x?}", dst, src, registers[src]);
                    unsafe {*(adt(dst, mem) as *mut u64) = registers[src]};
                    /*for (i, n) in registers[src].to_le_bytes().into_iter().enumerate() {
                        if !mem.writebyte(dst + (i as u64), n) {
                            unsafe {
                                libc::raise(11);
                            }
                        }
                    }*/
                }
                4 => unimplemented!(),
                5 => unimplemented!(),
                6 => unimplemented!(),
                7 => unimplemented!(),
                _ => unimplemented!(),
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
        Box::new(|isn, _, registers, pc| {
            let src1 = ((isn & 0x000f8000) >> 15) as usize;
            let pf1 = registers[src1];
            let src2 = ((isn & 0x01f00000) >> 20) as usize;
            let mut pf2 = registers[src2];
            let fct = (isn & 0x00007000) >> 12;
            let dst = ((isn & 0x00000f80) >> 7) as usize;
            match fct {
                0 => {
                    if isn & 0x4000_0000 != 0 {
                        pf2 = 0 - pf2;
                    }
                    //println!("add/sub d%{},%{},%{} = 0x{:x?}", dst, src1, src2, pf1 + pf2);
                    utils::write_register_safe(registers, dst, pf1 + pf2);
                }
                1 => {
                    //println!("sll d%{},%{}={:x?},%{}=m{:x?} = 0x{:x?}", dst, src1, pf1, src2, pf2, pf1 << pf2);
                    utils::write_register_safe(registers, dst, pf1 << pf2)
                }
                2 => unimplemented!(),
                3 => unimplemented!(),
                4 => unimplemented!(),
                5 => unimplemented!(),
                6 => unimplemented!(),
                7 => unimplemented!(),
                _ => unimplemented!(),
            }
            *pc += 4;
        }),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|isn, _, registers, pc| {
            let rd = ((isn & 0x00000f80) >> 7) as usize;
            let val = (isn & 0xfffff000) as u64;
            //println!("lui %{},0x{:x?}", rd, utils::sign_extend_32(val));
            utils::write_register_safe(
                registers,
                rd,
                utils::sign_extend_32(val),
            );
            *pc += 4;
        }),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|isn, _, registers, pc| {
            let src1 = ((isn & 0x000f8000) >> 15) as usize;
            let pf1 = registers[src1];
            let src2 = ((isn & 0x01f00000) >> 20) as usize;
            let mut pf2 = registers[src2];
            let fct = (isn & 0x00007000) >> 12;
            let dst = ((isn & 0x00000f80) >> 7) as usize;
            match fct {
                0 => {
                    if isn & 0x4000_0000 != 0 {
                        pf2 = 0 - pf2;
                    }
                    //println!("addw/subw d%{},%{},%{} = 0x{:x?}", dst, src1, src2, pf1 as u32 + (pf2 as u32));
                    utils::write_register_safe(registers, dst, utils::sign_extend_32(((pf1 as u32) + (pf2 as u32)) as u64));
                }
                1 => unimplemented!(),
                2 => unimplemented!(),
                3 => unimplemented!(),
                4 => unimplemented!(),
                5 => unimplemented!(),
                6 => unimplemented!(),
                7 => unimplemented!(),
                _ => unimplemented!(),
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
        Box::new(|isn, _, registers, pc| {
            let imm = utils::sign_extend_13((((isn & 0x8000_0000) >> 19)
                | ((isn & 0x7e00_0000) >> 20)
                | ((isn & 0x0000_0f00) >> 7)
                | ((isn & 0x0000_0080) << 4)) as u64);
            let fct = (isn & 0x00007000) >> 12;
            let rs2 = ((isn & 0x01f0_0000) >> 20) as usize;
            let rs1 = ((isn & 0x000f_8000) >> 15) as usize;
            let pf2 = registers[rs2];
            let pf1 = registers[rs1];
            //println!("branch {}, 1=%{}={:x?},2=%{}=0x{:x?}, imm={}, npc=0x{:x?}", fct, rs1, pf1, rs2, pf2, imm as i64, *pc + imm);
            match fct {
                0 => {
                    if (pf1 == pf2) {
                        *pc += imm;
                        return;
                    }
                },
                1 => {
                    if (pf1 != pf2) {
                        *pc += imm;
                        return;
                    }
                },
                2 => unimplemented!(),
                3 => unimplemented!(),
                4 => {
                    if ((pf1 as i64) < (pf2 as i64)) {
                        *pc += imm;
                        return;
                    }
                },
                5 => unimplemented!(),
                6 => unimplemented!(),
                7 => unimplemented!(),
                _ => unimplemented!()
            }
            *pc += 4;
        }),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|isn, _, registers, pc| {
            let imm = ((isn & 0xfff00000) >> 20) as u64;
            let src = ((isn & 0x000f8000) >> 15) as usize;
            let fct = (isn & 0x00007000) >> 12;
            let dst = ((isn & 0x00000f80) >> 7) as usize;
            let prefetch = registers[src];
            match fct {
                0 => {
                    //println!("jalr d%{},%{}=0x{:x?},{} = 0x{:x?}", dst, src, prefetch, utils::sign_extend_12(imm) as i64, prefetch + utils::sign_extend_12(imm));
                    let target = (prefetch + utils::sign_extend_12(imm));
                    if target & 1 != 0 {
                        panic!("attempted invalid jalr");
                    }
                    utils::write_register_safe(registers, dst, *pc + 4);
                    *pc = target;
                    return;
                }
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
        Box::new(|isn, _, registers, pc| {
            // opcode = 1101111 - J-type
            let rd = (isn & 0x00000f80) >> 7;
            utils::write_register_safe(registers, rd as usize, *pc + 4);
            let imm = ((isn & 0x8000_0000) >> 11)
                | ((isn & 0x7fe0_0000) >> 20)
                | ((isn & 0x0010_0000) >> 9)
                | (isn & 0x000f_f000);
            //println!("jal $0x{:x?},%{}", *pc + utils::sign_extend_21(imm as u64), rd);
            *pc = *pc + utils::sign_extend_21(imm as u64);
        }),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|_, _, _, _| unimplemented!()),
        Box::new(|isn, mem, registers, pc| {
            // opcode = 1110011 - R-type (SYSTEM)
            let imm = ((isn & 0xfff00000) >> 20) as u64;
            let fct = (isn & 0x00007000) >> 12;
            if fct == 0 {
                if imm == 0 {
                    //println!("ecall {}", registers[17]);
                    // ECALL
                    // TODO: probably should separate this into its own file
                    // TODO: use a lookup table instead of match
                    match registers[17] {
                        64 => {
                            let fd = registers[10];
                            let ptr: *mut libc::c_void = adt(registers[11], mem);
                            let n = registers[12];
                            //let mut v: Vec<u8> = Vec::with_capacity(n as usize);
                            /*for i in 0..n>>3 {
                                if let Some(a) = mem.readdword(ptr+i<<3) {
                                    v.extend_from_slice(&a.to_le_bytes());
                                } else {
                                    unsafe {libc::raise(11);}
                                }
                            }
                            for i in ptr+(n & 0xffff_ffff_ffff_fff8)..ptr+n {
                                if let Some(a) = mem.readbyte(i) {
                                    v.push(a);
                                } else {
                                    unsafe {libc::raise(11);}
                                }
                            }*/
                            unsafe {libc::write(fd as i32, ptr, n as usize);}
                        }
                        93 => std::process::exit(registers[10] as i32),
                        _ => unimplemented!(),
                    }
                } else if imm == 1 {
                    // EBREAK
                    unimplemented!()
                } else {
                    unimplemented!()
                }
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
    ];
    // TODO split into threads for multiprocessing
    loop {
        // No compressed instruction support
        let isn = unsafe {*(adt(pc, mema) as *const u32)};
        //println!("pc=0x{:x?}", pc);
        opcode_table[isn as usize & 0x7f](isn, mema, &mut registers, &mut pc);
    }
}

#[inline(always)]
fn adt(addr: u64, mema: *mut libc::c_void) -> *mut libc::c_void {
    (if addr & (1 << 38) != 0 {
        unsafe {mema.byte_offset((addr - stack_bottom) as isize)}
    } else {
        unsafe {mema.byte_offset(addr as isize)}
    }) as *mut libc::c_void
}
