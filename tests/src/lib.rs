#[cfg(test)]
mod tests {
    use core::assert_eq;
    use std::process::{Command, ExitStatus};

    #[test]
    fn hello() {
        let output = Command::new("../samples/hello")
            .output()
            .expect("Failed to execute process");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "hi there\n");
    }

    #[test]
    fn hello_c() {
        let output = Command::new("../samples/hello-c")
            .output()
            .expect("Failed to execute process");
        assert!(String::from_utf8_lossy(&output.stdout).contains("hi there"));
    }

    #[test]
    fn hello_args_one_arg() {
        let output = Command::new("../samples/hello-args")
            .arg("test")
            .output()
            .expect("Failed to execute process");

        assert!(String::from_utf8_lossy(&output.stdout).contains("Hello test !"));
    }

    #[test]
    fn hello_args_no_arg() {
        let output = Command::new("../samples/hello-args")
            .output()
            .expect("Failed to execute process");
        assert!(String::from_utf8_lossy(&output.stdout).contains("Missing name :/"));
    }

    #[test]
    fn hello_dl() {
        let output = Command::new("../samples/hello-dl")
            .output()
            .expect("Failed to execute process");
        assert!(String::from_utf8_lossy(&output.stdout).contains("hi there"));
    }

    #[test]
    fn hello_env() {
        let output = Command::new("../samples/hello-env")
            .env("NAME", "test")
            .output()
            .expect("Failed to execute process");
        assert!(String::from_utf8_lossy(&output.stdout).contains("Hello test !"));
    }

    #[test]
    fn hello_init_array() {
        let output = Command::new("../samples/hello-init-array")
            .output()
            .expect("Failed to execute process");
        assert!(String::from_utf8_lossy(&output.stdout).contains("hi there"));
    }

    #[test]
    fn hello_math() {
        let output = Command::new("../samples/hello-math")
            .output()
            .expect("Failed to execute process");
        assert!(String::from_utf8_lossy(&output.stdout).contains("1.414214"));
    }

    #[test]
    fn hello_mov_pie() {
        let output = Command::new("../samples/hello-mov-pie")
            .output()
            .expect("Failed to execute process");
        assert!(String::from_utf8_lossy(&output.stdout).contains("hi there"));
    }

    #[test]
    fn hello_pie() {
        let output = Command::new("../samples/hello-pie")
            .output()
            .expect("Failed to execute process");
        assert!(String::from_utf8_lossy(&output.stdout).contains("hi there"));
    }

    #[test]
    fn seccomp_allowed() {
        let output = Command::new("../samples/seccomp-allowed")
            .output()
            .expect("Failed to execute process");
        assert!(String::from_utf8_lossy(&output.stdout).contains("hi there"));
    }

    #[test]
    fn seccomp_forbidden() {
        let status = Command::new("../samples/seccomp-forbidden")
            .status()
            .expect("Failed to execute process");
        assert_eq!(status.success(), false);
    }
}
