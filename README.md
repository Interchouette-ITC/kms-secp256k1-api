# kms-secp256k1-api

AWS KMS-backed HTTP API for secp256k1 key operations (Ethereum, Cosmos, Casper) and Docker deployment.

Canonical repo: [Interchouette-ITC/kms-secp256k1-api](https://github.com/Interchouette-ITC/kms-secp256k1-api).

## Docs

| Doc | Description |
| --- | --- |
| [`docs/README.md`](docs/README.md) | Product overview, API, Docker, env |
| [`docs/Tutorial.md`](docs/Tutorial.md) | Tutorial |
| [`CHANGELOG.md`](CHANGELOG.md) | Semver notes |
| [`docker/README.md`](docker/README.md) | Image tags and release playbook |
| `make doc` → rustdoc | [GitHub Pages](https://interchouette-itc.github.io/kms-secp256k1-api/kms_secp256k1_api) |

## Quick start

```bash
make build
make test
make docker-build
make docker-run-test
```

Swagger UI (when running): `http://localhost:<APP_PORT>/api`

## Docker images

| Registry | Image |
| --- | --- |
| Docker Hub | `interchouette/kms-secp256k1-api`, `interchouette/kms-localstack` |
| Personal GHCR | `ghcr.io/groussac/kms-secp256k1-api`, `ghcr.io/groussac/kms-localstack` |
| Org GHCR | `ghcr.io/interchouette-itc/kms-secp256k1-api`, `ghcr.io/interchouette-itc/kms-localstack` |

```bash
docker pull interchouette/kms-secp256k1-api:dev
make docker-build-dev && make docker-push-dev   # :dev when you want (local)
# GitHub Actions → "CI/CD Image dev" (workflow_dispatch) for :dev
# GitHub Release tag vX.Y.Z → pushes :X.Y.Z and :latest (+ binary)
make version-show
```

Details: [`docker/README.md`](docker/README.md).

## Release playbook

1. `make version-show` (or `make version-bump-patch` / `version-set VERSION=x.y.z`)
2. Update [`CHANGELOG.md`](CHANGELOG.md); merge to `dev`
3. Create a GitHub Release on the org repo with tag **`v$(APP_VERSION)`** (must equal `Cargo.toml`)
4. `release.yml` publishes API + LocalStack `:version` / `:latest` to Hub and both GHCR registries, and attaches the Linux binary (WASM is embedded; optional `WASM_PATH` / on-disk `./wasm/wasm.wasm` override)

Not published to crates.io.

## License

MIT. See [`LICENSE`](LICENSE).
