#[derive(Debug)]
pub enum FoldError {
    InvalidSectionCast { expected: u32, actual: u32 },
    MissingLinkedSection,
    OutOfBounds,
    InvalidString,
}
