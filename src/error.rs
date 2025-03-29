use alloc::ffi::CString;

#[derive(Debug)]
pub enum FoldError {
    InvalidSectionCast { expected: u32, actual: u32 },
    MissingLinkedSection,
    SymbolNotFound(CString),
    OutOfBounds,
    InvalidString,
}
