use rustix::runtime;

/// Exit status code. See also [`exit`].
pub enum Exit {
    Success,
    Error,
}

/// Exit with an error code saying that something went wrong.
pub fn exit_error() -> ! {
    exit(Exit::Error);
}

/// Exit with the appropriate error code.
pub fn exit(status: Exit) -> ! {
    let value = match status {
        Exit::Success => 0,
        Exit::Error => 1,
    };

    runtime::exit_group(value);
}
