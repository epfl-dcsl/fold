use alloc::boxed::Box;

use crate::error::FoldError;

#[derive(Debug)]
pub enum SysvError {
    FoldError(FoldError),
    Other,
}

impl From<SysvError> for Box<dyn core::fmt::Debug> {
    fn from(value: SysvError) -> Self {
        Box::new(value)
    }
}

impl From<FoldError> for SysvError {
    fn from(value: FoldError) -> Self {
        SysvError::FoldError(value)
    }
}

impl From<FoldError> for Box<dyn core::fmt::Debug> {
    fn from(value: FoldError) -> Self {
        Box::new(SysvError::FoldError(value))
    }
}
