#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

use fold::{chain, Fold};

#[chain]
fn chain(chain: Fold) -> Fold {
    chain
}
