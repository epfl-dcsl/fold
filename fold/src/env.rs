use alloc::vec::Vec;
use core::ffi::{c_char, CStr};
use core::fmt;

// ——————————————————————————— Auxiliary Vectors ———————————————————————————— //

#[derive(Debug)]
#[repr(C)]
/// An entry in the auxiliary vector.
///
/// The auxiliary vector conveys information from the operating system to the application about the execution context
/// (see the [Linux documentation](<https://refspecs.linuxfoundation.org/LSB_1.3.0/IA64/spec/auxiliaryvector.html>)).
pub struct Auxv {
    /// Type of the entry.
    pub typ: AuxvType,
    /// Value of the entry.
    pub value: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
/// Type of an auxiliary vector ([`Auxv`]) entry.
pub struct AuxvType(u64);

impl AuxvType {
    /// Marks end of auxiliary vector list.
    pub const NULL: Self = Self(0);
    /// Address of the first program header in memory.
    pub const PHDR: Self = Self(3);
    /// Number of program headers.
    pub const PHNUM: Self = Self(5);
    /// Address where the interpreter (dynamic loader) is mapped.
    pub const BASE: Self = Self(7);
    /// Entry point of program.
    pub const ENTRY: Self = Self(9);
    // TODO: complete with missing types
    // (<https://refspecs.linuxfoundation.org/LSB_1.3.0/IA64/spec/auxiliaryvector.html>).
}

// —————————————————————————————— Environment ——————————————————————————————— //

/// Stores the environment of the process.
pub struct Env {
    /// CLI arguments given to the process.
    pub args: Vec<&'static CStr>,
    /// List of environment variables of the process, stored in `key=value` format. TODO: change to a tuple of
    /// `(key, value)` or even a [`alloc::collections::BTreeMap`] ?
    pub envp: Vec<&'static CStr>,
    /// Auxiliary vector.
    pub auxv: &'static [Auxv],
    /// Pointer to the start of the null-terminated argument array.
    pub raw_argv: usize,
    /// Pointer to the start of the null-terminated environments variable array.
    pub raw_envp: usize,
}

impl Env {
    /// Construct an [`Env`] from the data given by the kernel at the start of execution.
    ///
    /// # Safety
    /// `argv` must be a pointer to the start of the `argv` array.
    pub unsafe fn from_argv(argv: usize) -> Self {
        let (args, ptr) = Self::collect_strings(argv as *const _);
        let ptr = ptr.add(1);
        let raw_envp = ptr as usize;
        let (envp, ptr) = Self::collect_strings(ptr);
        let ptr = ptr.add(1);
        let auxv = Self::collect_auxv(ptr as *const _);
        Env {
            args,
            envp,
            auxv,
            raw_argv: argv,
            raw_envp,
        }
    }

    unsafe fn collect_strings(
        base: *const *const c_char,
    ) -> (Vec<&'static CStr>, *const *const c_char) {
        let mut strings = Vec::new();
        let mut base = base;

        loop {
            let ptr = *base;

            // Array is null terminated
            if ptr.is_null() {
                break;
            }

            strings.push(CStr::from_ptr(ptr));
            base = base.add(1);
        }

        (strings, base)
    }

    unsafe fn collect_auxv(base: *const Auxv) -> &'static [Auxv] {
        let mut n = 0;

        loop {
            let ptr = base.add(n);

            // Array is null terminated
            if (*ptr).typ == AuxvType::NULL {
                break;
            } else {
                n += 1;
            }
        }

        core::slice::from_raw_parts(base, n)
    }
}

// ———————————————————————————————— Display ————————————————————————————————— //

impl fmt::Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Env {{ args: [")?;
        for (idx, arg) in self.args.iter().enumerate() {
            if idx != 0 {
                write!(f, ", ")?;
            }

            if let Ok(arg) = arg.to_str() {
                write!(f, "{arg}")?;
            } else {
                write!(f, "<not utf-8>")?;
            }
        }
        write!(f, "], envp: [")?;
        for (idx, env) in self.envp.iter().enumerate() {
            if idx != 0 {
                write!(f, ", ")?;
            }

            if let Ok(env) = env.to_str() {
                write!(f, "{env}")?;
            } else {
                write!(f, "<not utf-8>")?;
            }
        }
        write!(f, "], auxv: [")?;
        for (idx, aux) in self.auxv.iter().enumerate() {
            if idx != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{aux:x?}")?;
        }
        write!(f, "]}}")
    }
}

impl fmt::Debug for AuxvType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NULL => write!(f, "NULL"),
            Self::PHDR => write!(f, "PHDR"),
            Self::PHNUM => write!(f, "PHNUM"),
            Self::BASE => write!(f, "BASE"),
            Self::ENTRY => write!(f, "ENTRY"),
            _ => write!(f, "<unknown>"),
        }
    }
}
