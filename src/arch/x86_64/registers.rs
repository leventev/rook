use alloc::fmt;

use super::{
    gdt::{segment_selector, GDT_KERNEL_CODE, GDT_KERNEL_DATA, GDT_USER_CODE, GDT_USER_DATA},
    Rflags,
};

#[repr(C, packed(16))]
#[derive(Clone, Copy, Debug)]
pub struct GeneralRegisters {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rbp: u64,
}

#[repr(C, packed(16))]
#[derive(Clone, Copy, Debug)]
pub struct IretRegisters {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[repr(C, packed(16))]
#[derive(Clone, Copy, Debug)]
pub struct SegmentSelectors {
    pub es: u64,
    pub ds: u64,
    pub fs: u64,
    pub gs: u64,
    pub ss: u64,
    pub cs: u64,
}

#[repr(C, packed(16))]
#[derive(Clone, Copy, Debug)]
pub struct RegisterState {
    pub general: GeneralRegisters,
    pub selectors: SegmentSelectors,
    pub rflags: u64,
    pub rip: u64,
    pub rsp: u64,
}

#[repr(C, packed(16))]
#[derive(Clone, Copy, Debug)]
pub struct InterruptRegisters {
    pub general: GeneralRegisters,
    pub iret: IretRegisters,
}

unsafe impl Sync for RegisterState {}

impl GeneralRegisters {
    pub const fn zero() -> Self {
        Self {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rbp: 0,
        }
    }
}

impl SegmentSelectors {
    pub const fn new_kernel() -> Self {
        Self {
            es: segment_selector(GDT_KERNEL_DATA, 0),
            ds: segment_selector(GDT_KERNEL_DATA, 0),
            fs: segment_selector(GDT_KERNEL_DATA, 0),
            gs: segment_selector(GDT_KERNEL_DATA, 0),
            cs: segment_selector(GDT_KERNEL_CODE, 0),
            ss: segment_selector(GDT_KERNEL_DATA, 0),
        }
    }

    pub const fn new_user() -> Self {
        Self {
            es: segment_selector(GDT_USER_DATA, 3),
            ds: segment_selector(GDT_USER_DATA, 3),
            fs: segment_selector(GDT_USER_DATA, 3),
            gs: segment_selector(GDT_USER_DATA, 3),
            cs: segment_selector(GDT_USER_CODE, 3),
            ss: segment_selector(GDT_USER_DATA, 3),
        }
    }

    pub const fn zero() -> Self {
        Self {
            es: 0,
            ds: 0,
            fs: 0,
            gs: 0,
            cs: 0,
            ss: 0,
        }
    }
}

impl RegisterState {
    pub const fn new_kernel() -> Self {
        Self {
            general: GeneralRegisters::zero(),
            selectors: SegmentSelectors::new_kernel(),
            rflags: Rflags::THREAD_DEFAULT.bits,
            rip: 0,
            rsp: 0,
        }
    }

    pub const fn new_user() -> Self {
        Self {
            general: GeneralRegisters::zero(),
            selectors: SegmentSelectors::new_user(),
            rflags: Rflags::THREAD_DEFAULT.bits,
            rip: 0,
            rsp: 0,
        }
    }

    pub const fn zero() -> Self {
        Self {
            general: GeneralRegisters::zero(),
            selectors: SegmentSelectors::zero(),
            rflags: Rflags::empty().bits,
            rip: 0,
            rsp: 0,
        }
    }
}

impl fmt::Display for GeneralRegisters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rax: u64 = self.rax;
        let rbx: u64 = self.rbx;
        let rcx: u64 = self.rcx;
        let rdx: u64 = self.rdx;
        let rsi: u64 = self.rsi;
        let rdi: u64 = self.rdi;
        let r8: u64 = self.r8;
        let r9: u64 = self.r9;
        let r10: u64 = self.r10;
        let r11: u64 = self.r11;
        let r12: u64 = self.r12;
        let r13: u64 = self.r13;
        let r14: u64 = self.r14;
        let r15: u64 = self.r15;
        let rbp: u64 = self.rbp;

        writeln!(
            f,
            "RAX={rax:0>16x} RBX={rbx:0>16x} RCX={rcx:0>16x} RDX={rdx:0>16x}"
        )?;
        writeln!(
            f,
            "RSI={rsi:0>16x} RDI={rdi:0>16x}  R8={r8:0>16x}  R9={r9:0>16x}"
        )?;
        writeln!(
            f,
            "R10={r10:0>16x} R11={r11:0>16x} R12={r12:0>16x} R13={r13:0>16x}"
        )?;
        writeln!(f, "R14={r14:0>16x} R15={r15:0>16x} RBP={rbp:0>16x}")
    }
}

impl fmt::Display for SegmentSelectors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let es: u64 = self.es;
        let ds: u64 = self.ds;
        let fs: u64 = self.fs;
        let gs: u64 = self.gs;
        let ss: u64 = self.ss;
        let cs: u64 = self.cs;

        write!(
            f,
            "ES={es:0>4x} DS={ds:0>4x} FS={fs:0>4x} GS={gs:0>4x} SS={ss:0>4x} CS={cs:0>4x}"
        )
    }
}

impl fmt::Display for RegisterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rip: u64 = self.rip;
        let rflags: Rflags = Rflags::from_bits(self.rflags).unwrap();
        let rsp: u64 = self.rsp;

        write!(f, "{}", self.general)?;

        writeln!(f, "RIP={rip:0>16x} RSP={rsp:0>16x}")?;
        writeln!(f, "RFLAGS={rflags:0>16x}({:?})", rflags)?;

        write!(f, "{}", self.selectors)
    }
}

impl fmt::Display for IretRegisters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rip = self.rip;
        let rsp = self.rsp;
        let rflags = Rflags::from_bits(self.rflags).unwrap();
        let cs = self.cs;
        let ss = self.ss;

        writeln!(
            f,
            "RIP={rip:0>16x} RSP={rsp:0>16x} RFLAGS={rflags:0>16x}({:?})",
            rflags
        )?;
        writeln!(f, "SS={ss:0>4x} CS={cs:0>4x}")
    }
}

impl fmt::Display for InterruptRegisters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //writeln!(f, "{}", self.general)?;
        write!(f, "{}", self.iret)
    }
}
