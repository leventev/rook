use alloc::slice;

use crate::{
    arch::x86_64::paging::{PML1Flags, PML2Flags, PML3Flags, PML4Flags},
    mm::{
        phys::{self, PageDescriptorManager, FRAME_SIZE},
        PhysAddr,
    },
    utils,
};

use super::{PAGE_ENTRIES, PML4};

macro_rules! define_get_pml {
    ($name: ident, $fl: ty) => {
        pub fn $name(&self, table_phys: PhysAddr, index: u64) -> Option<(PhysAddr, $fl)> {
            let virt_addr = table_phys.virt_addr();

            let table = unsafe {
                slice::from_raw_parts(virt_addr.get() as *const u64, PAGE_ENTRIES as usize)
            };

            match table[index as usize] {
                0 => None,
                val => {
                    let phys = PhysAddr::new(val & 0x000ffffffffff000);
                    let flags = <$fl>::from_bits(val & 0xFFF).unwrap();

                    Some((phys, flags))
                }
            }
        }
    };
}

macro_rules! define_map_pml {
    ($name: ident, $fl: ty, $big_page: expr) => {
        pub fn $name(
            &self,
            pgm: &mut PageDescriptorManager,
            table_phys: PhysAddr,
            index: u64,
            phys: PhysAddr,
            flags: $fl,
        ) {
            let virt_addr = table_phys.virt_addr();
            let table = unsafe {
                slice::from_raw_parts_mut(virt_addr.get() as *mut u64, PAGE_ENTRIES as usize)
            };

            let ent = phys.get() | flags.bits();
            let unmap = ent == 0;

            if pgm.initialized {
                if $big_page {
                    for i in 0..512 {
                        let addr = phys + PhysAddr::new(i * FRAME_SIZE as u64);
                        if unmap {
                            pgm.dec_used_count(addr);
                        } else {
                            pgm.inc_used_count(addr);
                        }
                    }
                } else {
                    if unmap {
                        pgm.dec_used_count(phys);
                    } else {
                        pgm.inc_used_count(phys);
                    }
                }
            }

            table[index as usize] = ent;
        }
    };
}

macro_rules! define_get_or_map_pml {
    ($name: ident, $fl: ty, $get: ident, $map: ident) => {
        pub fn $name(
            &self,
            pgm: &mut PageDescriptorManager,
            table_phys: PhysAddr,
            index: u64,
            flags: $fl,
        ) -> PhysAddr {
            match self.$get(table_phys, index) {
                Some(ent) => {
                    assert!(ent.1.difference(flags) != <$fl>::DIRTY);
                    ent.0
                }
                None => {
                    let phys = phys::alloc();
                    utils::zero_page(phys.virt_addr().get() as *mut u64);
                    self.$map(pgm, table_phys, index, phys, flags);
                    phys
                }
            }
        }
    };
}

impl PML4 {
    define_get_pml!(get_pml4, PML4Flags);
    define_get_pml!(get_pml3, PML3Flags);
    define_get_pml!(get_pml2, PML2Flags);
    define_get_pml!(get_pml1, PML1Flags);

    define_map_pml!(map_pml4, PML4Flags, false);
    define_map_pml!(map_pml3, PML3Flags, false);
    define_map_pml!(map_pml2, PML2Flags, false);
    define_map_pml!(map_pml2_2mib, PML2Flags, true);
    define_map_pml!(map_pml1, PML1Flags, false);

    define_get_or_map_pml!(get_or_map_pml4, PML4Flags, get_pml4, map_pml4);
    define_get_or_map_pml!(get_or_map_pml3, PML3Flags, get_pml3, map_pml3);
    define_get_or_map_pml!(get_or_map_pml2, PML2Flags, get_pml2, map_pml2);
    define_get_or_map_pml!(get_or_map_pml1, PML1Flags, get_pml1, map_pml1);
    define_get_or_map_pml!(get_or_map_pml2_2mib, PML2Flags, get_pml2, map_pml2_2mib);
}
