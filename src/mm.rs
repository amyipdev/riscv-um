// SPDX-License-Identifier: GPL-2.0-or-later

use crate::utils;

use std::sync::{Arc, Mutex};

// 38-bit memory mapping
pub(crate) struct MemoryMap {
    pub l1: Box<[Option<L2Table>; 32768]>,
    pub total_pages_alloc: usize,
}
impl MemoryMap {
    pub(crate) fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            l1: Box::new([const { None }; 32768]),
            total_pages_alloc: 0,
        }))
    }
    /// Pass any address within the page to allocate it.
    pub(crate) fn allocate_known_page(&mut self, page: u64) -> bool {
        let l1i = (page >> 24) as usize;
        if self.l1[l1i].is_none() {
            self.l1[l1i] = Some(L2Table {
                ents: Box::new([const { None }; 4096]),
            });
        }
        let l2a = self.l1[l1i].as_mut().unwrap();
        let l2i = ((page >> 12) & 0xfff) as usize;
        if l2a.ents[l2i].is_none() {
            l2a.ents[l2i] = Some([0; 4096]);
            self.total_pages_alloc += 1;
            return true;
        }
        return false;
    }
    /// Pass an address range to get all pages within it allocated.
    pub(crate) fn allocate_address_range(&mut self, start_address: u64, size: u64) -> bool {
        let base = start_address >> 12;
        let top = ((start_address + size) >> 12) + if size << 52 != 0 {1} else {0};
        for i in base..top {
            if !self.allocate_known_page(i << 12) {
                return false;
            }
        }
        return true;
    }
    #[inline(always)]
    pub(crate) fn writebyte(&mut self, addr: u64, byte: u8) -> bool {
        if let Some(ref mut l2) = self.l1[(addr >> 24) as usize] {
            if let Some(ref mut l3) = l2.ents[((addr >> 12) & 0xfff) as usize] {
                l3[(addr & 0xfff) as usize] = byte;
                return true;
            } else {
                return false;
            }
        } else {
            return false;
        }
    }
    #[inline(always)]
    pub(crate) fn readbyte(&self, addr: u64) -> Option<u8> {
        if let Some(ref l2) = self.l1[(addr >> 24) as usize] {
            if let Some(ref l3) = l2.ents[((addr >> 12) & 0xfff) as usize] {
                return Some(l3[(addr & 0xfff) as usize]);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    #[inline(always)]
    pub(crate) fn readhword(&self, addr: u64) -> Option<u16> {
        if addr & 1 != 0 && addr & 0xfff > 0xffe {
            let lh = self.readbyte(addr);
            let uh = self.readbyte(addr + 1);
            if let Some(b0) = lh {
                if let Some(b1) = uh {
                    return Some((b1 as u16) << 8 + b0 as u16);
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        if let Some(ref l2) = self.l1[(addr >> 24) as usize] {
            if let Some(ref l3) = l2.ents[((addr >> 12) & 0xfff) as usize] {
                let by = (addr & 0xfff) as usize;
                return Some(utils::extract_u16_from_page(l3, by));
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    #[inline(always)]
    pub(crate) fn readword(&self, addr: u64) -> Option<u32> {
        if addr & 3 != 0 && addr & 0xfff > 0xffc {
            let lh = self.readhword(addr);
            let uh = self.readhword(addr + 2);
            if let Some(b0) = lh {
                if let Some(b1) = uh {
                    return Some((b1 as u32) << 16 + b0 as u32);
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        if let Some(ref l2) = self.l1[(addr >> 24) as usize] {
            if let Some(ref l3) = l2.ents[((addr >> 12) & 0xfff) as usize] {
                let by = (addr & 0xfff) as usize;
                return Some(utils::extract_u32_from_page(l3, by));
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    #[inline(always)]
    pub(crate) fn readdword(&self, addr: u64) -> Option<u64> {
        if addr & 3 != 0 && addr & 0xfff > 0xffc {
            let lh = self.readword(addr);
            let uh = self.readword(addr + 4);
            if let Some(b0) = lh {
                if let Some(b1) = uh {
                    return Some((b1 as u64) << 16 + b0 as u64);
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        if let Some(ref l2) = self.l1[(addr >> 24) as usize] {
            if let Some(ref l3) = l2.ents[((addr >> 12) & 0xfff) as usize] {
                let by = (addr & 0xfff) as usize;
                return Some(utils::extract_u64_from_page(l3, by));
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
}

pub(crate) struct L2Table {
    pub ents: Box<[Option<[u8; 4096]>; 4096]>,
}
