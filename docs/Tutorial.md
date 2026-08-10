**KMS Secp256k1 API** - See base docker commands at https://github.com/Interchouette-ITC/kms-secp256k1-api

**A high-performance custodial software wallet middleware for secure blockchain key operations.**

Rust-based API service that provides key management, signing, and verification using **secp256k1**.
Supports multiple blockchains including **Ethereum**, **Cosmos**, and **Casper**.

This service is designed to run securely inside Docker and connects to **AWS KMS** (currently the only supported KMS backend).

Docker Hub / GHCR image tags and publish flow: [`docker/README.md`](../docker/README.md).

<details>
  <summary><strong><code>Overview</code></strong></summary>

A fast and secure Rust-based API service that acts as a custodial wallet for blockchain apps.
It enables cryptographic transactions (generation, signing, listing, deletion) via AWS KMS using the secp256k1 elliptic curve, while keeping private keys secure.

</details>

<details>
  <summary><strong><code>Key Features</code></strong></summary>

- Multi-chain support: Works with Casper, Ethereum, and Cosmos networks.
- Secure key management: All signing and key operations are handled within AWS KMS - private keys never leave the AWS infrastructure.
- Robust API: Exposes endpoints for key life cycle and signing - `/createKey`, `/deleteKey`, `/listKeys`, `/signTransaction`, `/signTransactionHash`, `/verifySignature`.
- Selective endpoints: Security-sensitive routes like deletion and listing can be disabled at runtime.
- Auditable and secure: Fine-grained access control, logging, and no local key storage.
</details>

<details>
  <summary><strong><code>Features</code></strong></summary>

- ✅ Generate secp256k1 keys
- ✅ Sign and verify messages and transactions
- ✅ Support for multiple blockchain formats:
  - Ethereum
  - Cosmos
  - Casper
- ✅ AWS KMS integration for production-ready security
- ✅ Easy containerized deployment
- ✅ Easy Testing or CI/CD deployment
</details>

<details>
  <summary><strong><code>Architecture Overview</code></strong></summary>

[Your App] → [API Layer (this service)] → [AWS KMS (HSM secp256k1)] → [Signed Transactions] → [Blockchain Network]

The API abstracts cryptographic details, presenting your app with a secure and simple interface.

</details>

<details>
  <summary><strong><code>Highlights & Endpoints</code></strong></summary>

- Endpoints:
  - POST /createKey - create a new keypair
  - POST /signTransactionHash - sign a hex transaction hash (query param: keys)
  - POST /signTransaction - sign a JSON transaction (query param: keys)
  - GET /verifySignature - verify signatures (optional via KMS)
  - GET /listKeys - list keys (can be disabled)
  - DELETE /deleteKey - delete a key (can be disabled)
  - GET / - simple hello/health

  - OpenAPI / Swagger UI:
  - Swagger UI: /docs on `http://localhost:<APP_PORT>/docs`
  - OpenAPI JSON: /api-doc/openapi.json on `http://localhost:<APP_PORT>/api-doc/openapi.json`

- Feature flags via env:
  - TESTING_MODE, AWS_MODE, DELETE_MODE, LIST_MODE, BLOCKCHAIN_MODE, etc.

- Port:
  - Controlled by APP_PORT (e.g., 4001 for test, 4000 for prod)
  </details>

<details>
  <summary><strong><code>Usage Scenarios</code></strong></summary>

Ideal for:

- Enterprise blockchain systems requiring private key custody
- DeFi platforms needing secure signing
- NFT marketplaces with regulatory requirements
- Multi-chain applications
- Regulatory-compliant solutions with auditability
</details>

<details>
  <summary><strong><code>Security-First Design</code></strong></summary>

- Private keys remain protected in the AWS KMS (Customer keys)
- No local key storage, reducing risk of compromise
- Critical endpoints (`/deleteKey`, `/listKeys`) can be disabled at runtime for extra safety
</details>

<details>
  <summary><strong><code>API Overview</code></strong></summary>

### Health Check

```bash
curl http://localhost:4000/
```

### Create a Keypair

```bash
curl -X POST http://localhost:4000/createKey
```

### Sign Transaction Hash

```bash
curl -X POST "http://localhost:4000/signTransactionHash?keys=<key1>&keys=<key2>" \
 -H "Content-Type: text/plain" \
 -d "0x..."
```

### Sign JSON Transaction

```bash
curl -X POST "http://localhost:4000/signTransaction?keys=<key1>&keys=<key2>" \
 -H "Content-Type: application/json" \
 -d '{"field":"value"}'
```

### Verify Signature

```bash
curl -X GET "http://localhost:4000/verifySignature?key=<key>&transaction_hash=<hex>&signature=<sig>&via_kms=<true|false>"
```

### List Keys (if enabled)

```bash
curl -X GET http://localhost:4000/listKeys
```

### Delete Key (if enabled)

```bash
curl -X DELETE "http://localhost:4000/deleteKey?key=<address_or_pubkey>"
```

</details>

---

<details>
  <summary><strong><code>Quick Setup</code></strong></summary>

### Build Docker Image

```bash
docker build -t yourusername/kms-secp256k1-api:latest .
```

### Run in Test Mode (port 4001)

```bash
docker run --rm -it \
 -p 4001:4001 \
 -e TESTING_MODE=true \
 yourusername/kms-secp256k1-api:latest
```

### Run in Production Mode (port 4000)

```bash
docker run --rm -it \
 -p 4000:4000 \
 --env-file .env \
 yourusername/kms-secp256k1-api:latest
```

## Docker run

Run in **test** mode (no real chain interaction; useful for integration and local dev).
Adjust the env values as needed.

```bash
docker run --rm -it \
 -p 4001:4001 \
 -e APP_PORT=4001 \
 -e TESTING_MODE=true \
 -e BLOCKCHAIN_MODE=casper \
 -e DELETE_MODE=true \
 -e LIST_MODE=true \
# ...
# or add env file
 --env-file .env.test \
 interchouette/kms-secp256k1-api:latest
```

Run in **production** mode (example: port 4000).
If you use AWS KMS, set AWS creds/region and set AWS_MODE=true.

```bash
docker run --rm -it \
 -p 4000:4000 \
 --env-file .env \
 interchouette/kms-secp256k1-api:latest
```

</details>

<details>
  <summary><strong><code>Docker Compose (examples)</code></strong></summary>

**Test compose** (maps 4001:4001 and loads `.env.test`):

```yaml
services:
kms-secp256k1-api:
  container_name: kms-secp256k1-api-test
  image: interchouette/kms-secp256k1-api:latest
  env_file: - .env.test
  ports: - "4001:4001"
```

**Prod compose** (maps 4000:4000 and loads `.env`):

```yaml
services:
kms-secp256k1-api:
  container_name: kms-secp256k1-api
  image: interchouette/kms-secp256k1-api:latest
  env_file: - .env
  ports: - "4000:4000"
```

</details>

<details>
  <summary><strong><code>Environment Variables</code></strong></summary>

Environment variables can be set in .env file

| Variable       | Description                             | Default |
| -------------- | --------------------------------------- | ------- |
| TESTING_MODE   | Enable test keys (mocked, for dev only) | `true`  |
| AWS_REGION     | AWS region                              | none    |
| KMS_CREATE_ID  | AWS IAM create key                      | none    |
| KMS_CREATE_KEY | AWS IAM Create key                      | none    |

...

Core configuration:

- APP_PORT - HTTP port inside the container (e.g., `4001` for test, `4000` for prod)
- TESTING_MODE - true or false (enables test mode with dummy KMS values)
- BLOCKCHAIN_MODE - one of: casper, ethereum, cosmos
- AWS_MODE - true to enable AWS KMS; false to disable (currently only AWS is supported)
- AWS_REGION - e.g., eu-west-3 (required if AWS_MODE=true)
- DOTENV_DISABLE - if set, skip loading `.env` (use in containers with a task role)
- DELETE_MODE - true enables DELETE /deleteKey endpoint
- LIST_MODE - true enables GET /listKeys endpoint
- ETH_CHAIN_ID - Ethereum chain ID (used for signing transactions), defaults to wasmd `1`
- COSMOS_HRP - Bech32 HRP prefix for Cosmos addresses, defaults to wasmd `cosmos`
- COSMOS_REST_URL - URL for the Cosmos REST node, defaults to wasmd `http://localhost:1317/cosmos/auth/v1beta1/accounts/`
- COSMOS_CHAIN_ID - Cosmos chain ID, defaults to wasmd `testing`

KMS credentials:

- Set `KMS_CREATE_ID`/`KMS_CREATE_KEY` and `KMS_SIGN_ID`/`KMS_SIGN_KEY` for static IAM user keys (LocalStack / lab). Optional `KMS_DELETE_*` / `KMS_LIST_*` when those modes are on.
- Leave those unset on AWS to use the task role / instance profile (all enabled ops share that role). Set `DOTENV_DISABLE` and do not set `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`.

**Example of `.env` (production) file:**

```
APP_PORT=4000
TESTING_MODE=false
BLOCKCHAIN_MODE=casper

AWS_REGION=eu-west-3

DELETE_MODE=false
LIST_MODE=true

KMS_CREATE_ID=your_IAM_User
KMS_CREATE_KEY=your_IAM_Key
KMS_SIGN_ID=your_IAM_User
KMS_SIGN_KEY=your_IAM_Key
KMS_DELETE_ID=your_IAM_User
KMS_DELETE_KEY=your_IAM_Key
KMS_LIST_ID=your_IAM_User
KMS_LIST_KEY=your_IAM_Key
```

</details>

<details>
  <summary><strong><code>Development</code></strong></summary>

### Clone and Build Locally

```bash
git clone https://github.com/Interchouette-ITC/kms-secp256k1-api.git
cd kms-secp256k1-api
cargo build
```

### Run Tests

```bash
cargo test -- --nocapture
```

</details>

<details>
  <summary><strong><code>Make Commands</code></strong></summary>

You can use the provided `Makefile` for common tasks:

| Command                      | Description                                                |
| ---------------------------- | ---------------------------------------------------------- |
| `make build`                 | Build the Rust project (`cargo build`)                     |
| `make test`                  | Run tests with output (`cargo test -- --nocapture`)        |
| `make lint`                  | Run Clippy linter with strict rules                        |
| `make check-lint`            | Auto-fix Clippy lints where possible                       |
| `make docker-build`          | Build Hub image `:latest` and `:<Cargo version>`           |
| `make docker-build-dev`      | Build and tag `:dev` (Hub + GHCR names)                    |
| `make docker-build-no-cache` | Build Docker image without cache                           |
| `make docker-run-test`       | Run Docker Compose test environment (maps port 4001)       |
| `make docker-run`            | Run Docker Compose production environment (maps port 4000) |
| `make docker-stop`           | Stop the production container                              |

### Example

Build and run locally:

```bash
make build
make test
make docker-build
make docker-run-test
```

</details>

## License

MIT License. Free to use and modify.
