//! Make / Docker lifecycle helpers (parity with repository Makefile targets).

use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::paths::{
    api_log_path, api_pid_path, repo_root, run_dir, API_PROD_CONTAINER, API_TEST_CONTAINER,
    API_TEST_LOCALSTACK_CONTAINER, COMPOSE_TEST, COMPOSE_TEST_LOCALSTACK, DEFAULT_API_URL,
    DEFAULT_MCP_URL, DEFAULT_TEST_API_URL, LOCALSTACK_CONTAINER, LOCALSTACK_TEST_CONTAINER,
};

fn run(cmd: &str, args: &[&str]) -> (i32, String, String) {
    match Command::new(cmd)
        .args(args)
        .current_dir(repo_root())
        .output()
    {
        Ok(out) => (
            out.status.code().unwrap_or(1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ),
        Err(e) => (127, String::new(), e.to_string()),
    }
}

fn format_cmd(label: &str, code: i32, out: &str, err: &str) -> String {
    if code != 0 {
        return format!(
            "{label} failed ({code}):\n{}",
            if err.is_empty() { out } else { err }
        );
    }
    format!("{label} ok.\n{out}{err}").trim().to_string()
}

fn make_args(target: &str, features: Option<&str>) -> Vec<String> {
    let mut args = vec![target.to_string()];
    if let Some(f) = features {
        if !f.is_empty() {
            args.push(format!("FEATURES={f}"));
        }
    }
    args
}

fn make_target(target: &str, features: Option<&str>) -> String {
    let args = make_args(target, features);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, out, err) = run("make", &arg_refs);
    format_cmd(&format!("make {}", args.join(" ")), code, &out, &err)
}

fn compose(file: &str, args: &[&str]) -> (i32, String, String) {
    let mut full = vec!["compose", "-f", file];
    full.extend_from_slice(args);
    run("docker", &full)
}

fn docker_ps_filter(name: &str) -> String {
    let (code, out, err) = run(
        "docker",
        &[
            "ps",
            "-a",
            "--filter",
            &format!("name={name}"),
            "--format",
            "{{.Names}}\t{{.Status}}\t{{.Ports}}",
        ],
    );
    if code != 0 {
        return format!("docker ps failed: {err}");
    }
    if out.trim().is_empty() {
        format!("{name}: (not found)")
    } else {
        out.trim().to_string()
    }
}

fn curl_get(url: &str) -> String {
    let (code, out, err) = run("curl", &["-sS", "-m", "3", "-o", "-", "-w", "\nHTTP %{http_code}", url]);
    if code != 0 {
        return format!("curl {url} failed: {err}");
    }
    out.trim().to_string()
}

fn docker_logs_tail(container: &str, lines: u32) -> String {
    let (code, out, err) = run(
        "docker",
        &["logs", "--tail", &lines.to_string(), container],
    );
    let text = if out.is_empty() {
        err
    } else {
        format!("{out}{err}")
    };
    if code != 0 && text.trim().is_empty() {
        return format!("(no docker logs yet for {container})");
    }
    format!("--- docker logs {container} (last {lines}) ---\n{text}")
}

/// Make help + MCP tool map.
pub fn help() -> String {
    let make_help = make_target("help", None);
    format!(
        "{make_help}\n\n\
         MCP mirrors local-dev Make (not push/version-bump):\n\
         - kms_build / kms_build_release / kms_check / kms_lint / kms_test / kms_verify\n\
         - kms_test_localstack\n\
         - kms_docker_build* / kms_docker_run* / kms_docker_stop*\n\
         - kms_docker_build_localstack / kms_docker_run_localstack / kms_docker_stop_localstack\n\
         - kms_docker_inspect / kms_status / kms_version_show\n\
         - kms_api_start / kms_api_stop (host cargo, mock)\n\
         - kms_stack_start / kms_stack_stop (LocalStack + API compose on :4001)\n\
         HTTP tools: kms_hello / kms_create_key / kms_sign_* / kms_verify_signature /\n\
         kms_delete_key / kms_list_keys / kms_openapi\n\
         MCP HTTP: {DEFAULT_MCP_URL}\n\
         Default API: {DEFAULT_API_URL} (test compose: {DEFAULT_TEST_API_URL})"
    )
}

pub fn build(features: Option<&str>) -> String {
    make_target("build", features)
}

pub fn build_release(features: Option<&str>) -> String {
    make_target("build-release", features)
}

pub fn check(features: Option<&str>) -> String {
    make_target("check", features)
}

pub fn lint(features: Option<&str>) -> String {
    make_target("lint", features)
}

pub fn test_mock(features: Option<&str>) -> String {
    make_target("test", features)
}

pub fn verify(features: Option<&str>) -> String {
    make_target("verify", features)
}

pub fn test_localstack(features: Option<&str>) -> String {
    make_target("test-localstack", features)
}

pub fn docker_build() -> String {
    make_target("docker-build", None)
}

pub fn docker_build_dev() -> String {
    make_target("docker-build-dev", None)
}

pub fn docker_build_no_cache() -> String {
    make_target("docker-build-no-cache", None)
}

pub fn docker_build_localstack() -> String {
    make_target("docker-build-localstack", None)
}

pub fn docker_build_localstack_dev() -> String {
    make_target("docker-build-localstack-dev", None)
}

/// `make docker-run` (prod compose, detached).
pub fn docker_run() -> String {
    make_target("docker-run", None)
}

/// Detached test compose (Make `docker-run-test` is foreground — MCP adds `-d`).
pub fn docker_run_test() -> String {
    let (code, out, err) = compose(
        COMPOSE_TEST,
        &["up", "-d", "--no-build", "--force-recreate"],
    );
    let mut text = format_cmd("docker compose test up -d", code, &out, &err);
    if code == 0 {
        thread::sleep(Duration::from_secs(2));
        text.push('\n');
        text.push_str(&docker_logs_tail(API_TEST_CONTAINER, 40));
        text.push_str(&format!(
            "\nAPI URL hint: set KMS_API_URL={DEFAULT_TEST_API_URL}"
        ));
    }
    text
}

pub fn docker_stop() -> String {
    make_target("docker-stop", None)
}

/// Also tear down test compose containers if present.
pub fn docker_stop_all_api() -> String {
    let stop_prod = make_target("docker-stop", None);
    let (c1, o1, e1) = compose(COMPOSE_TEST, &["down", "-v", "--remove-orphans"]);
    let test = format_cmd("compose test down", c1, &o1, &e1);
    format!("{stop_prod}\n{test}")
}

pub fn docker_run_localstack() -> String {
    make_target("docker-run-localstack", None)
}

pub fn docker_stop_localstack() -> String {
    make_target("docker-stop-localstack", None)
}

pub fn docker_inspect() -> String {
    make_target("docker-inspect", None)
}

pub fn version_show() -> String {
    make_target("version-show", None)
}

/// LocalStack + API via `docker-compose.test-localstack.yml` (API on :4001).
pub fn stack_start() -> String {
    let mut parts = Vec::new();
    parts.push(docker_build_localstack());
    parts.push(docker_build());
    let (code, out, err) = compose(
        COMPOSE_TEST_LOCALSTACK,
        &["up", "-d", "--force-recreate"],
    );
    parts.push(format_cmd(
        "compose test-localstack up -d",
        code,
        &out,
        &err,
    ));
    if code == 0 {
        parts.push("Waiting for LocalStack health...".into());
        for i in 1..=60 {
            let (c, o, _) = run(
                "docker",
                &[
                    "inspect",
                    "--format={{.State.Health.Status}}",
                    LOCALSTACK_TEST_CONTAINER,
                ],
            );
            let status = if c == 0 {
                o.trim().to_string()
            } else {
                "starting".into()
            };
            if status == "healthy" {
                parts.push(format!("LocalStack healthy after ~{i} checks"));
                break;
            }
            if i == 60 {
                parts.push("LocalStack did not become healthy".into());
            }
            thread::sleep(Duration::from_secs(2));
        }
        thread::sleep(Duration::from_secs(2));
        parts.push(docker_logs_tail(API_TEST_LOCALSTACK_CONTAINER, 40));
        parts.push(format!(
            "Set KMS_API_URL={DEFAULT_TEST_API_URL} for HTTP tools"
        ));
    }
    parts.join("\n\n")
}

pub fn stack_stop() -> String {
    let (code, out, err) = compose(
        COMPOSE_TEST_LOCALSTACK,
        &["down", "-v", "--remove-orphans"],
    );
    format_cmd("compose test-localstack down", code, &out, &err)
}

fn api_release_bin() -> std::path::PathBuf {
    repo_root().join("target/release/kms-secp256k1-api")
}

/// Ensure `target/release/kms-secp256k1-api` exists for the requested features.
/// Prefer the release binary over `cargo run` so the MCP pid is the server (not cargo)
/// and cold CI does not race a 90s compile against readiness probes.
fn ensure_api_release_bin(features: &str) -> Result<std::path::PathBuf, String> {
    let bin = api_release_bin();
    let (code, out, err) = run(
        "cargo",
        &[
            "build",
            "--release",
            "--no-default-features",
            "--features",
            features,
            "--quiet",
        ],
    );
    if code != 0 {
        return Err(format!(
            "cargo build --release --features {features} failed ({code}):\n{err}{out}"
        ));
    }
    if !bin.is_file() {
        return Err(format!("missing binary after build: {}", bin.display()));
    }
    Ok(bin)
}

/// Start host API in background with mock KMS (`TESTING_MODE=true`).
/// Builds (if needed) then execs `target/release/kms-secp256k1-api` — never long-lived `cargo run`.
pub fn api_start(features: Option<&str>, port: Option<u16>) -> String {
    if let Some(existing) = read_api_pid() {
        if process_alive(existing) {
            return format!("host API already running (pid {existing})");
        }
    }
    if let Err(e) = fs::create_dir_all(run_dir()) {
        return format!("cannot create run dir: {e}");
    }
    let port = port.unwrap_or(4000);
    let features = features.unwrap_or("casper");
    let bin = match ensure_api_release_bin(features) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let log = api_log_path();
    let log_file = match fs::File::create(&log) {
        Ok(f) => f,
        Err(e) => return format!("cannot create log {}: {e}", log.display()),
    };
    let err_file = match log_file.try_clone() {
        Ok(f) => f,
        Err(e) => return format!("cannot clone log handle: {e}"),
    };

    let mut cmd = Command::new(&bin);
    cmd.current_dir(repo_root())
        .env("TESTING_MODE", "true")
        .env("AWS_MODE", "true")
        .env("DELETE_MODE", "true")
        .env("LIST_MODE", "true")
        .env("BLOCKCHAIN_MODE", "casper")
        .env("APP_PORT", port.to_string())
        .env("KMS_CREATE_ID", "test")
        .env("KMS_CREATE_KEY", "test")
        .env("KMS_SIGN_ID", "test")
        .env("KMS_SIGN_KEY", "test")
        .env("KMS_DELETE_ID", "test")
        .env("KMS_DELETE_KEY", "test")
        .env("KMS_LIST_ID", "test")
        .env("KMS_LIST_KEY", "test")
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(err_file));

    match cmd.spawn() {
        Ok(child) => {
            let pid = child.id();
            if let Err(e) = fs::write(api_pid_path(), pid.to_string()) {
                return format!("spawned pid {pid} but failed to write pid file: {e}");
            }
            // Wait briefly for listen
            thread::sleep(Duration::from_secs(2));
            let hello = curl_get(&format!("http://127.0.0.1:{port}/"));
            format!(
                "host API started pid={pid} port={port} features={features}\n\
                 bin={}\n\
                 log={}\n\
                 hello probe:\n{hello}\n\
                 Set KMS_API_URL=http://127.0.0.1:{port}",
                bin.display(),
                log.display()
            )
        }
        Err(e) => format!("failed to spawn {}: {e}", bin.display()),
    }
}

pub fn api_stop() -> String {
    let Some(pid) = read_api_pid() else {
        return "no host API pid file (nothing to stop)".into();
    };
    let (code, out, err) = run("kill", &[&pid.to_string()]);
    let _ = fs::remove_file(api_pid_path());
    if code != 0 {
        // try kill -9
        let (c2, _, e2) = run("kill", &["-9", &pid.to_string()]);
        if c2 != 0 {
            return format!("kill {pid} failed: {err}{e2}");
        }
    }
    format!("stopped host API pid={pid}\n{out}{err}").trim().to_string()
}

fn read_api_pid() -> Option<u32> {
    let text = fs::read_to_string(api_pid_path()).ok()?;
    text.trim().parse().ok()
}

fn process_alive(pid: u32) -> bool {
    let (code, _, _) = run("kill", &["-0", &pid.to_string()]);
    code == 0
}

/// Container + HTTP health snapshot.
pub fn status() -> String {
    let mut lines = vec![
        format!("repo_root={}", repo_root().display()),
        format!("KMS_API_URL={}", crate::paths::api_base_url()),
        format!("MCP HTTP={DEFAULT_MCP_URL}"),
        String::new(),
        "=== containers ===".into(),
        docker_ps_filter(LOCALSTACK_CONTAINER),
        docker_ps_filter(LOCALSTACK_TEST_CONTAINER),
        docker_ps_filter(API_PROD_CONTAINER),
        docker_ps_filter(API_TEST_CONTAINER),
        docker_ps_filter(API_TEST_LOCALSTACK_CONTAINER),
        String::new(),
        "=== API probes ===".into(),
        format!("GET {DEFAULT_API_URL}/"),
        curl_get(&format!("{DEFAULT_API_URL}/")),
        format!("GET {DEFAULT_TEST_API_URL}/"),
        curl_get(&format!("{DEFAULT_TEST_API_URL}/")),
    ];
    if let Some(pid) = read_api_pid() {
        lines.push(String::new());
        lines.push(format!(
            "host API pid={pid} alive={}",
            process_alive(pid)
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_env::ENV_LOCK;
    use std::env;
    use std::fs;

    #[test]
    fn help_mentions_parity_tools() {
        let _g = ENV_LOCK.lock().unwrap();
        let mut dir = env::temp_dir();
        dir.push(format!("kms-mcp-help-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("Makefile"),
            "help:\n\t@echo kms-secp256k1-api targets\n",
        )
        .unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.0.0\"\nedition=\"2021\"\n",
        )
        .unwrap();
        env::set_var("KMS_API_ROOT", &dir);
        let text = help();
        assert!(text.contains("kms_build"), "{text}");
        assert!(text.contains("kms_stack_start"), "{text}");
        env::remove_var("KMS_API_ROOT");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_runs_with_fixture() {
        let _g = ENV_LOCK.lock().unwrap();
        let mut dir = env::temp_dir();
        dir.push(format!("kms-mcp-status-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("docker")).unwrap();
        fs::write(dir.join("Makefile"), "help:\n\t@echo ok\n").unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.0.0\"\nedition=\"2021\"\n",
        )
        .unwrap();
        env::set_var("KMS_API_ROOT", &dir);
        let text = status();
        assert!(text.contains("repo_root="), "{text}");
        assert!(text.contains("containers"), "{text}");
        env::remove_var("KMS_API_ROOT");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn compose_paths_exist_in_real_repo() {
        use crate::paths::{COMPOSE_LOCALSTACK, COMPOSE_PROD};
        let root = repo_root();
        if root.join(COMPOSE_PROD).is_file() {
            assert!(root.join(COMPOSE_TEST).is_file());
            assert!(root.join(COMPOSE_LOCALSTACK).is_file());
            assert!(root.join(COMPOSE_TEST_LOCALSTACK).is_file());
        }
    }
}
