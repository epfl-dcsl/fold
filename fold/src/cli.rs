//! # Command Line Interface

use core::ffi::CStr;

use crate::env::Env;
use crate::exit::exit_error;
use crate::println;

pub struct Config {
    pub target: &'static CStr,
    pub env: Env,
}

/// Parse command line arguments.
pub fn parse(env: Env, loader_name: &str) -> Config {
    let args = &env.args;
    if args.is_empty() {
        log::error!("No target to execute");
        usage();
        exit_error();
    }

    let Some(target) = find_target(args, loader_name) else {
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
fn find_target(args: &[&'static CStr], loader_name: &str) -> Option<&'static CStr> {
    assert!(!args.is_empty());

    // If arg 0 is not self, then it is the target
    let bytes = args[0].to_bytes();
    if bytes.len() >= loader_name.len() {
        let suffix_range = (bytes.len() - loader_name.len())..bytes.len();
        if &bytes[suffix_range] != loader_name.as_bytes() {
            return Some(args[0]);
        }
    } else {
        return Some(args[0]);
    }

    // Otherwise, we are invoked directly, search forthe target
    args.get(1).copied()
}
