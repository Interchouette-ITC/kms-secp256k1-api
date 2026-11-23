use std::process::Command;

pub fn run_isolated(test: &str, ignored: bool, configure: impl FnOnce(&mut Command)) -> bool {
    const MARKER: &str = "KMS_MCP_ISOLATED_TEST";
    if std::env::var(MARKER).ok().as_deref() == Some(test) {
        return false;
    }

    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .arg("--exact")
        .arg(test)
        .arg("--nocapture")
        .env(MARKER, test)
        .env_remove("KMS_API_ROOT")
        .env_remove("KMS_HOST_ROOT")
        .env_remove("KMS_API_URL");
    if ignored {
        command.arg("--ignored");
    }
    configure(&mut command);
    let output = command.output().expect("run isolated test");
    assert!(
        output.status.success()
            && String::from_utf8_lossy(&output.stdout).contains("running 1 test"),
        "isolated test {test} failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
