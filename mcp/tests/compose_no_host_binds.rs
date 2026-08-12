//! Regression: product compose stays free of host bind volumes; MCP ops never
//! drive the MCP sidecar compose (host-started only).

use std::fs;
use std::path::PathBuf;

use kms_secp256k1_api_mcp::paths::PRODUCT_COMPOSE_FILES;

fn repo_docker_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("docker")
}

/// Short-syntax bind: `- /host:/container` or `- ./rel:/c` or `- $VAR:/c`.
fn is_host_bind_volume_line(line: &str) -> bool {
    let t = line.trim();
    if t.starts_with('#') || !t.starts_with('-') {
        return false;
    }
    let rest = t.trim_start_matches('-').trim();
    if !rest.contains(':') {
        return false;
    }
    let source = rest.split(':').next().unwrap_or("").trim();
    source.starts_with('/')
        || source.starts_with('.')
        || source.starts_with('$')
        || source.starts_with('~')
}

#[test]
fn product_compose_has_no_host_bind_volumes() {
    let docker = repo_docker_dir();
    for rel in PRODUCT_COMPOSE_FILES {
        let name = rel.strip_prefix("docker/").unwrap_or(rel);
        let path = docker.join(name);
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for (i, line) in text.lines().enumerate() {
            assert!(
                !is_host_bind_volume_line(line),
                "{}:{}: host bind volume not allowed in product compose (use ${{KMS_HOST_ROOT}} only after updating MCP guards): {line}",
                path.display(),
                i + 1
            );
        }
        // Stronger lock for today's layout: no volumes: key at all.
        assert!(
            !text.lines().any(|l| l.trim_start().starts_with("volumes:")),
            "{}: unexpected volumes: key — product stacks must not bind host paths; \
             if you add volumes, sources must be ${{KMS_HOST_ROOT}}/… and this test must be updated",
            path.display()
        );
    }
}

#[test]
fn mcp_sidecar_compose_is_not_a_product_compose_constant() {
    for rel in PRODUCT_COMPOSE_FILES {
        assert!(
            !rel.contains("mcp"),
            "PRODUCT_COMPOSE_FILES must not include MCP sidecar compose: {rel}"
        );
    }
}

#[test]
fn mcp_src_never_references_compose_mcp() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut hits = Vec::new();
    for entry in fs::read_dir(&src).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        if text.contains("docker-compose.mcp") || text.contains("COMPOSE_MCP") {
            hits.push(path.display().to_string());
        }
    }
    assert!(
        hits.is_empty(),
        "MCP lifecycle must not call docker-compose.mcp.yml (host-started only): {hits:?}"
    );
}
