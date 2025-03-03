//! # Command Line Interface

use core::ffi::CStr;

use crate::env::Env;
use crate::exit::exit_error;
use crate::println;

const SELF: &[u8] = b"fold";

pub struct Config {
    pub target: &'static CStr,
    env: Env,
}

/// Parse command line arguments.
pub fn parse(env: Env) -> Config {
    let args = &env.args;
    if args.is_empty() {
        log::error!("No target to execute");
        usage();
        exit_error();
    }

    let Some(target) = find_target(args) else {
        log::error!("No target to execute");
        usage();
        exit_error();
    };

    Config { target, env }
}

/// Print help.
fn usage() {
    println!("Spidl Dynamic Loader\n");
    println!("Usage: spidl <target> [args]");
}

/// Find the program to load.
fn find_target(args: &[&'static CStr]) -> Option<&'static CStr> {
    assert!(!args.is_empty());

    // If arg 0 is not self, then it is the target
    let bytes = args[0].to_bytes();
    if bytes.len() >= SELF.len() {
        let suffix_range = (bytes.len() - SELF.len())..bytes.len();
        if &bytes[suffix_range] != SELF {
            return Some(args[0]);
        }
    } else {
        return Some(args[0]);
    }

    // Otherwise, we are invoked directly, search forthe target
    args.get(1).copied()
}
