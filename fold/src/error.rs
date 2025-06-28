use alloc::ffi::CString;

#[derive(Debug)]
/// Errors that may originate from Fold's internal working.
pub enum FoldError {
    InvalidSectionCast { expected: u32, actual: u32 },
    MissingLinkedSection,
    SymbolNotFound(CString),
    OutOfBounds,
    InvalidString,
}
