# KMS secp256k1 API

# KMS secp256k1 API

A high-performance **custodial software wallet** built in Rust that serves as middleware between your blockchain applications and secure key storage on KMS providers. This API provides cryptographic operations for multiple blockchain networks using secp256k1 elliptic curve cryptography while keeping private keys secure in cloud-based key management systems.

Native support for Casper, Ethereum, or Cosmos networks

⚠ Only supporting **AWS** as KMS storage for now, some more KMS provider **secp256k1** will be integrated (TODO)

## Swagger UI

![KMS secp256k1 API](https://github.com/gRoussac/kms-secp256k1-api/blob/dev/docs/images/Swagger-UI.png)

<details>
  <summary><strong><code>What This Custodial KMS API Does</code></strong></summary>

### **Blockchain Perspective**

This software acts as a **custodial wallet service** that:

- **Generates cryptographic keypairs** for blockchain operations
- **Signs transactions, deploys, and messages** without exposing private keys
- **Manages key lifecycle** (creation, deletion, listing) through secure KMS providers
- **Supports multiple blockchain networks** (Casper, Ethereum, Cosmos) with their specific cryptographic requirements
</details>

<details>
  <summary><strong><code>Tutorial / Doc</code></strong></summary>

[KMS secp256k1 API Tutorial/Doc](https://github.com/gRoussac/kms-secp256k1-api/blob/dev/docs/Tutorial.md)

</details>

<details>
  <summary><strong><code>Middleware Architecture</code></strong></summary>

The API serves as a **secure bridge** between your applications and cloud-based key storage:

- **Application Layer**: Your blockchain apps make HTTP requests to sign transactions
- **API Layer**: This service handles the cryptographic operations
- **KMS Layer**: Private keys are securely stored in AWS KMS (currently supporting secp256k1)
- **Blockchain Layer**: Signed transactions are returned to your application
</details>

<details>
  <summary><strong><code>Key Security Benefits</code></strong></summary>

- **Private keys never leave the KMS**: All signing operations happen within AWS KMS
- **No local key storage**: Eliminates risk of local key compromise
- **Audit trails**: All key operations are logged and traceable
- **Access control**: Fine-grained permissions for different operations (sign, create, delete, list)
- **Hardware security**: Leverages AWS KMS hardware security modules (HSMs)
</details>

<details>
  <summary><strong><code>How It Works</code></strong></summary>

1. **Key Generation**: Creates keypairs in AWS KMS with secp256k1 curve
2. **Signing Process**:
   - Your app sends transaction hash/message to the API
   - API requests AWS KMS to sign using the stored private key
   - AWS KMS performs the cryptographic operation internally
   - Signed result is returned to your application
3. **Key Management**: Keys can be listed, deleted, and managed through the API
</details>

<details>
  <summary><strong><code>Use Cases</code></strong></summary>

- **DeFi Applications**: Secure transaction signing for decentralized finance
- **NFT Marketplaces**: Safe key management for digital asset transactions
- **Enterprise Blockchain**: Corporate blockchain solutions requiring key custody
- **Multi-chain Applications**: Single API for multiple blockchain networks
- **Compliance Requirements**: Meeting regulatory requirements for key custody
</details>

<details>
  <summary><strong><code>Route Security Implementation</code></strong></summary>

The API implements conditional route loading based on security configuration:

```rust
// Routes are only loaded when corresponding modes are enabled
if config.is_delete_mode() {
    app = app.route("/deleteKey", delete(delete_key));
}

if config.is_list_mode() {
    app = app.route("/listKeys", get(list_keys));
}
```

**Security Benefits**:

- **DELETE_MODE disabled**: `/deleteKey` endpoint is completely unavailable
- **LIST_MODE disabled**: `/listKeys` endpoint is completely unavailable
- **Defense in depth**: Even if authentication is bypassed, dangerous endpoints don't exist
- **Runtime security**: Routes are not compiled into the binary when disabled
</details>

<details>
  <summary><strong><code>Best Practices for Key Management</code></strong></summary>

#### **Store Public Keys Locally After Generation**

**⚠️ CRITICAL RECOMMENDATION**: After generating a keypair with `/createKey`, immediately store the returned `public_key` and `address` in your application's local database or configuration.

**Why this is essential**:

- **Eliminates need for LIST_MODE**: You don't need to query the API for keys you already have
- **Reduces attack surface**: No need to expose the `/listKeys` endpoint
- **Better performance**: No network calls to retrieve key information
- **Enhanced security**: Keys are only exposed during initial generation

#### **Implementation Pattern**

```bash
# 1. Generate keypair
curl -X POST http://localhost:4000/createKey

# 2. Store response locally
{
  "public_key": "04a1b2c3...",
  "address": "casper1abc..."
}

# 3. Use stored public key for signing operations
curl -X POST http://localhost:4000/signTransactionHash \
  -d '{"key_id": "your-stored-key-id", "hash": "..."}'
```

#### **Avoid LIST_MODE - Security Best Practice**

**🚫 NEVER enable LIST_MODE in production unless absolutely necessary**

**Reasons to avoid listing keys**:

- **Information disclosure**: Reveals all keys in your system
- **Attack vector**: Public keys can be used to attempt signing operations
- **Audit complexity**: Harder to track who accessed which keys
- **Compliance issues**: May violate data minimization principles

**Alternative approach**:

- Generate keys and store metadata locally
- Implement key rotation without listing
- Use key aliases or tags in your application
- Maintain key registry in your own database
</details>

<details>
  <summary><strong><code>TESTING_MODE - Mock API for Development & CI/CD</code></strong></summary>

#### **What is TESTING_MODE?**

`TESTING_MODE=true` enables a **completely mocked API** that simulates all KMS operations without requiring any connection to AWS KMS services. This allows you to test your blockchain applications against a realistic API interface without incurring AWS costs or requiring production credentials.

#### **How TESTING_MODE Works**

- **Mock Services**: Uses `MockCasperKeysService`, `MockEthereumKeysService`, or `MockCosmosKeysService`
- **No AWS Connection**: Completely isolated from AWS KMS - no network calls, no credentials needed
- **Deterministic Responses**: Generates predictable, testable responses for consistent testing
- **Full API Coverage**: All endpoints work exactly as they would in production
- **Local Development**: Perfect for development machines without AWS access

#### **Use Cases for TESTING_MODE**

##### **1. Local Development**

```bash
export TESTING_MODE=true
export BLOCKCHAIN_MODE=casper
cargo run
# API runs locally with mock services - no AWS needed
```

##### **2. CI/CD Pipeline Testing**

```bash
# In your CI/CD pipeline
export TESTING_MODE=true
export BLOCKCHAIN_MODE=ethereum
cargo test
# Run integration tests without AWS credentials
```

##### **3. Test Environment Deployment**

```bash
# Deploy to test/staging environment
export TESTING_MODE=true
export BLOCKCHAIN_MODE=cosmos
docker run -e TESTING_MODE=true -e BLOCKCHAIN_MODE=cosmos kms-secp256k1-api
```

##### **4. Offline Development**

- **No internet required** - works completely offline
- **No AWS account needed** - perfect for open source contributors
- **No costs incurred** - free testing and development

#### **Testing_MODE vs Production**

| Aspect          | TESTING_MODE=true      | TESTING_MODE=false               |
| --------------- | ---------------------- | -------------------------------- |
| **AWS KMS**     | ❌ No connection       | ✅ Full integration              |
| **Credentials** | ❌ Not required        | ✅ Required                      |
| **Costs**       | ❌ Free                | ✅ AWS charges apply             |
| **Network**     | ❌ Offline capable     | ✅ Internet required             |
| **Security**    | ⚠️ Mock data           | ✅ Real cryptographic operations |
| **Use Case**    | 🧪 Development/Testing | 🚀 Production                    |

#### **Implementation in Code**

```rust
// From lib.rs - service selection based on TESTING_MODE
let keys_service: Box<dyn KeysServiceTrait> = if config.is_testing_mode() {
    if config.is_ethereum_mode() {
        Box::new(MockEthereumKeysService::new(config.clone(), crypto_service))
    } else if config.is_casper_mode() {
        Box::new(MockCasperKeysService::new(config.clone(), crypto_service))
    } else if config.is_cosmos_mode() {
        Box::new(MockCosmosKeysService::new(config.clone(), crypto_service))
    }
} else {
    // Production services with real AWS KMS integration
    // ...
};
```

#### **Benefits for Development Teams**

- **Faster iteration**: No need to wait for AWS operations
- **Cost control**: No charges during development
- **Offline work**: Develop without internet connection
- **CI/CD friendly**: Automated testing without AWS setup
- **Team onboarding**: New developers can start immediately
- **Open source**: Contribute without AWS account
</details>

<details>
  <summary><strong><code>CRITICAL NETWORK SECURITY WARNING</code></strong></summary>

#### **⚠️ NEVER EXPOSE THIS API ON PUBLIC NETWORKS**

**This API software was designed for PRIVATE INFRASTRUCTURE ONLY and should NEVER be exposed to the public internet.**

#### **Why Public Exposure is Extremely Dangerous**

- **Anyone can sign with known public keys**: If an attacker discovers a public key, they can attempt to sign transactions
- **No built-in authentication**: The API has no authentication layer by default
- **Direct access to KMS operations**: Attackers can potentially access your AWS KMS keys
- **Financial risk**: Unauthorized transactions can result in loss of funds/assets
- **Compliance violations**: Public exposure may violate security and regulatory requirements

#### **MANDATORY AUTHENTICATION REQUIREMENTS**

**You SHALL/MUST implement an authentication guard before deploying this API on publci interfaces, any among those follwing are recommended:**

- **JWT Authentication**: Implement JWT token validation
- **API Key Authentication**: Use secure API keys with proper rotation
- **OAuth2/OIDC**: Enterprise-grade authentication systems
- **Network-level authentication**: VPN, IP whitelisting, or reverse proxy authentication
- **Rate limiting**: Prevent brute force attacks
- **Request signing**: HMAC-based request validation

#### **NETWORK DEPLOYMENT RECOMMENDATIONS**

**Preferred deployment scenarios:**

1. **Private VPC/Network**: Deploy within AWS VPC or private cloud network
2. **VPN Access Only**: Require VPN connection for API access
3. **Reverse Proxy**: Use nginx/traefik with authentication
4. **Load Balancer**: AWS ALB/NLB with authentication
5. **Service Mesh**: Istio/Linkerd for microservice security
6. **Zero Trust Network**: Implement zero-trust security model

#### **🚫 What NOT to Do**

- ❌ **Never expose on public IP addresses**
- ❌ **Never deploy without authentication**
- ❌ **Never use in public cloud without proper network isolation**
- ❌ **Never assume the API is secure by default**
- ❌ **Never skip security testing before production deployment**

#### **✅ What You SHALL/MUST Do**

- ✅ **Implement strong authentication (JWT, API keys, OAuth)**
- ✅ **Deploy in private networks only**
- ✅ **Use HTTPS/TLS encryption**
- ✅ **Implement proper access controls**
- ✅ **Regular security audits and penetration testing**
- ✅ **Monitor and log all API access**
</details>

<details>
  <summary><strong><code>Features</code></strong></summary>

- **Multi-Blockchain Support**: Native support for Casper, Ethereum, and Cosmos networks
- **AWS KMS Integration**: Secure key management through AWS Key Management Service
- **RESTful API**: Clean HTTP endpoints with OpenAPI/Swagger documentation
- **High Performance**: Built with Axum web framework and async/await
- **WASM Support**: WebAssembly integration for cross-platform compatibility (cryptographic operations)
- **Testing Mode**: Mock services for development and testing
- **Comprehensive Logging**: Structured logging with configurable levels
</details>

<details>
  <summary><strong><code>Architecture</code></strong></summary>

The API is built with a modular architecture:

- **Routes Layer**: HTTP endpoints and request/response handling
- **Services Layer**: Business logic for different blockchain networks
- **Crypto Layer**: Core cryptographic operations using secp256k1
- **AWS Integration**: Secure key storage and management
- **WASM Loader**: Cross-platform cryptographic operations
</details>

<details>
  <summary><strong><code>WASM Cryptographic Operations</code></strong></summary>

This API includes WebAssembly (WASM) integration for enhanced cryptographic operations. The WASM module is loaded from the `WASM_PATH` and provides additional cryptographic functionality beyond the native Rust implementations.

**Important Note**: The WASM binary file (`./wasm/wasm.wasm`) IS included in the repository, but the **source code that generates this WASM file is NOT open source** and is **NOT publicly available**. The proprietary source code contains specialized cryptographic algorithms and implementations that are not shared.

### WASM Features

- Enhanced cryptographic operations
- Cross-platform compatibility
- Optimized performance for specific algorithms
- Proprietary cryptographic implementations
</details>

<details>
  <summary><strong><code>Prerequisites</code></strong></summary>

- Rust 1.70+ with Cargo
- Docker (optional, for containerized deployment)
- AWS credentials (for production use)
</details>

<details>
  <summary><strong><code>Installation</code></strong></summary>

### From Source

```bash
# Clone the repository
git clone <repository-url>
cd kms-secp256k1-api

# Build the project
cargo build --release

# Run the server
cargo run --release
```

### Using Docker

```bash
# Build the Docker image
docker build -t kms-secp256k1-api .

# Run the container
docker run -p 4000:4000 kms-secp256k1-api
```

</details>

<details>
  <summary><strong><code>Configuration</code></strong></summary>

The API can be configured using environment variables. You can find an example configuration file in `.env.test` for testing purposes.

### 🔒 **Critical Security Configuration**

**⚠️ IMPORTANT: The following configuration options have significant security implications and should be carefully considered before enabling in production environments.**

### Core Configuration

| Variable          | Default  | Description                                                             |
| ----------------- | -------- | ----------------------------------------------------------------------- |
| `APP_PORT`        | `4000`   | Server port                                                             |
| `TESTING_MODE`    | `true`   | Enable testing mode with mock services (see Testing Mode section below) |
| `BLOCKCHAIN_MODE` | `casper` | Blockchain network (casper, ethereum, cosmos)                           |

### 🔐 **Security-Critical Configuration**

| Variable      | Default | Description                   | **Security Risk**                      |
| ------------- | ------- | ----------------------------- | -------------------------------------- |
| `DELETE_MODE` | `false` | Enable key deletion endpoints | **HIGH** - Can permanently delete keys |
| `LIST_MODE`   | `false` | Enable key listing endpoints  | **HIGH** - Exposes all available keys  |

#### **DELETE_MODE - High Risk**

- **What it does**: Enables the `/deleteKey` endpoint for key deletion
- **Security implications**:
  - Anyone with access can permanently delete cryptographic keys
  - **Loss of access to funds/assets** if keys are deleted
  - **Irreversible operation** - deleted keys cannot be recovered
- **Recommendation**:
  - **Keep disabled** for regular API users
  - **Enable only for AWS administrators** with proper access controls
  - Use AWS IAM policies to restrict deletion permissions

#### **LIST_MODE - High Risk**

- **What it does**: Enables the `/listKeys` endpoint to enumerate all available keys
- **Security implications**:
  - **Exposes all public keys** in the system
  - **Attack vector**: If an attacker knows a public key, they can attempt to sign with it
  - **Information disclosure** reveals the scope of your key management system
- **Recommendation**:
  - **Keep disabled** unless absolutely necessary
  - **Audit access** if enabled
  - Consider implementing key rotation instead of listing

### AWS KMS Configuration

| Variable         | Description                                |
| ---------------- | ------------------------------------------ |
| `AWS_MODE`       | Enable AWS KMS integration                 |
| `AWS_REGION`     | AWS region for KMS operations              |
| `KMS_SIGN_ID`    | AWS access key for signing operations      |
| `KMS_SIGN_KEY`   | AWS secret key for signing operations      |
| `KMS_CREATE_ID`  | AWS access key for key creation            |
| `KMS_CREATE_KEY` | AWS secret key for key creation            |
| `KMS_DELETE_ID`  | AWS access key for key deletion (optional) |
| `KMS_DELETE_KEY` | AWS secret key for key deletion (optional) |
| `KMS_LIST_ID`    | AWS access key for key listing (optional)  |
| `KMS_LIST_KEY`   | AWS secret key for key listing (optional)  |

### Blockchain-Specific Configuration

#### Ethereum

| Variable       | Default | Description       |
| -------------- | ------- | ----------------- |
| `ETH_CHAIN_ID` | `1`     | Ethereum chain ID |

#### Cosmos

| Variable          | Default                                               | Description                  |
| ----------------- | ----------------------------------------------------- | ---------------------------- |
| `COSMOS_CHAIN_ID` | `cosmoshub-4`                                         | Cosmos chain ID              |
| `COSMOS_HRP`      | `cosmos`                                              | Cosmos human-readable prefix |
| `COSMOS_REST_URL` | `http://localhost:1317/cosmos/auth/v1beta1/accounts/` | Cosmos REST endpoint         |

</details>

<details>
  <summary><strong><code>Cosmos-Specific Configuration & Requirements</code></strong></summary>

#### **Why Cosmos Needs Additional Configuration**

Unlike Ethereum and Casper, **Cosmos requires external blockchain data** to properly sign transactions. This is because Cosmos transactions need:

- **`account_number`**: Unique identifier for the account on the Cosmos chain
- **`sequence`**: Transaction counter that prevents replay attacks
- **`chain_id`**: Specific Cosmos network identifier

#### **Critical Cosmos Configuration Variables**

##### **`COSMOS_REST_URL` - Blockchain Data Source**

- **Purpose**: REST endpoint to fetch account information (`account_number` and `sequence`)
- **Default**: `http://localhost:1317/cosmos/auth/v1beta1/accounts/` (⚠️ **Local Development Only**)
- **Why it's needed**:
  - Cosmos transactions require `account_number` and `sequence` for signing
  - These values change with each transaction and must be fetched from the blockchain
  - Without this, transaction signing will fail

**⚠️ IMPORTANT**: The default REST URL points to `localhost:1317` which is only suitable for local development. **You MUST customize this in your `.env` file for production use** to point to the actual Cosmos network's REST endpoint.

##### **`COSMOS_CHAIN_ID` - Network Identifier**

- **Purpose**: Identifies which Cosmos network to use (e.g., `cosmoshub-4`, `osmosis-1`)
- **Default**: `cosmoshub-4`
- **Why it's needed**:
  - Prevents cross-chain replay attacks
  - Ensures transactions are signed for the correct network
  - Required for transaction validation

##### **`COSMOS_HRP` - Address Prefix**

- **Purpose**: Human-readable prefix for Cosmos addresses (e.g., `cosmos`, `osmo`, `atom`)
- **Default**: `cosmos`
- **Why it's needed**:
  - Converts public keys to proper Cosmos addresses
  - Different Cosmos networks use different prefixes
  - Ensures address compatibility with the target network

#### **How Cosmos Transaction Signing Works**

```rust
// From cosmos_keys_service.rs - the signing process requires:

// 1. Fetch account info from REST endpoint
let account = fetch_account_info(&key, &pubkey_base64, config).await?;

// 2. Extract account_number and sequence
let account_number = account.account_number;
let sequence = account.sequence;

// 3. Build AuthInfo with these values
let auth_info = build_auth_info(&public_key_bytes, sequence, &fee)?;

// 4. Create SignDoc with chain_id and sequence
let sign_doc = SignDoc::new(&tx_body, &auth_info, &chain_id, account.sequence)?;
```

#### **Cosmos vs Other Blockchains**

| Aspect                    | Ethereum        | Casper          | **Cosmos**                          |
| ------------------------- | --------------- | --------------- | ----------------------------------- |
| **Account Info**          | ❌ Not required | ❌ Not required | **✅ Required**                     |
| **REST Endpoint**         | ❌ Not needed   | ❌ Not needed   | **✅ Must be configured**           |
| **Chain ID**              | ✅ Required     | ✅ Required     | **✅ Required**                     |
| **Address Prefix**        | ❌ Fixed format | ❌ Fixed format | **✅ Configurable (HRP)**           |
| **Transaction Structure** | Simple          | Simple          | **Complex (requires account data)** |

#### **Cosmos Configuration Examples**

##### **Cosmos Hub (Mainnet)**

```bash
export COSMOS_CHAIN_ID="cosmoshub-4"
export COSMOS_HRP="cosmos"
export COSMOS_REST_URL="https://api.cosmos.network/cosmos/auth/v1beta1/accounts/"
```

##### **Osmosis Network**

```bash
export COSMOS_CHAIN_ID="osmosis-1"
export COSMOS_HRP="osmo"
export COSMOS_REST_URL="https://lcd.osmosis.zone/cosmos/auth/v1beta1/accounts/"
```

##### **Local Testnet**

```bash
export COSMOS_CHAIN_ID="testing"
export COSMOS_HRP="cosmos"
export COSMOS_REST_URL="http://localhost:1317/cosmos/auth/v1beta1/accounts/"
```

**⚠️ Production Warning**: Never use `localhost:1317` in production! Always configure `COSMOS_REST_URL` to point to the actual network's REST endpoint.

#### **Common Cosmos Issues & Solutions**

##### **Issue: "Failed to fetch account info"**

- **Cause**: `COSMOS_REST_URL` is incorrect or unreachable
- **Solution**: Verify the REST endpoint and network connectivity

##### **Issue: "Invalid chain_id"**

- **Cause**: `COSMOS_CHAIN_ID` doesn't match the target network
- **Solution**: Use the correct chain ID for your target network

##### **Issue: "Account not found"**

- **Cause**: Account doesn't exist on the blockchain yet
- **Solution**: The API handles this gracefully by creating a default account with `sequence: 0`

#### **Cosmos Transaction Flow**

1. **Parse Transaction**: JSON transaction with messages and fee
2. **Fetch Account Info**: Get `account_number` and `sequence` from REST endpoint
3. **Build SignDoc**: Create signing document with chain ID and sequence
4. **Sign Hash**: Sign the transaction hash using AWS KMS
5. **Construct TxRaw**: Build the final signed transaction
6. **Return Result**: JSON with signed transaction and broadcast request
</details>

<details>
  <summary><strong><code>Quick Start</code></strong></summary>

1. **Set up environment variables**:

```bash
export TESTING_MODE=true
export BLOCKCHAIN_MODE=casper
export APP_PORT=4000
```

2. **Start the server**:

```bash
cargo run
```

3. **Access the API**:

- API: http://localhost:4000
- Swagger UI: http://localhost:4000/api
- OpenAPI JSON: http://localhost:4000/api-doc/openapi.json
</details>

<details>
  <summary><strong><code>API Endpoints</code></strong></summary>

### Health Check

- `GET /` - Hello endpoint with version information

### Key Management

- `POST /createKey` - Create a new keypair
- `DELETE /deleteKey/{key_id}` - Delete a key
- `GET /listKeys` - List all available keys

### Cryptographic Operations

- `POST /signTransactionHash` - Sign a transaction hash
- `POST /signTransaction` - Sign a complete transaction
- `POST /verifySignature` - Verify a signature
</details>

<details>
  <summary><strong><code>Authentication</code></strong></summary>

The API supports multiple authentication modes:

- **Testing Mode**: No authentication required (default)
- **AWS Mode**: AWS credentials for KMS operations
- **Production Mode**: Full AWS KMS integration
</details>

<details>
  <summary><strong><code>Usage Examples</code></strong></summary>

### Create a Keypair

```bash
curl -X POST http://localhost:4000/createKey \
  -H "Content-Type: application/json"
```

Response:

```json
{
  "public_key": "04a1b2c3...",
  "address": "casper1abc..."
}
```

### Sign a Transaction Hash

```bash
curl -X POST http://localhost:4000/signTransactionHash \
  -H "Content-Type: application/json" \
  -d '{
    "key_id": "your-key-id",
    "hash": "transaction-hash-here"
  }'
```

### Verify a Signature

```bash
curl -X POST http://localhost:4000/verifySignature \
  -H "Content-Type: application/json" \
  -d '{
    "public_key": "04a1b2c3...",
    "hash": "transaction-hash-here",
    "signature": "signature-here"
  }'
```

</details>

<details>
  <summary><strong><code>Testing</code></strong></summary>

The API includes comprehensive testing capabilities:

```bash
# Run all tests
cargo test

# Run tests with output (recommended)
cargo test -- --nocapture

# Using Makefile (recommended)
make test

# Run specific test modules
cargo test --test create_keypair
cargo test --test delete_key
cargo test --test list_keys
cargo test --test hello

# Run blockchain-specific tests
cargo test --test casper
cargo test --test ethereum
cargo test --test cosmos
```

### Available Test Targets

Based on your Makefile and test structure:

- **`make test`** - Run all tests with output (equivalent to `cargo test -- --nocapture`)
- **`make lint`** - Run clippy linting with strict rules
- **`make check-lint`** - Auto-fix linting issues where possible
</details>

<details>
  <summary><strong><code>Development</code></strong></summary>

### Project Structure

```
src/
├── main.rs              # Application entry point
├── lib.rs               # Library configuration and app creation
├── config.rs            # Configuration management
├── constants.rs         # Application constants
├── routes.rs            # HTTP route definitions
├── services/            # Business logic services
│   ├── keys_service.rs      # Key service trait
│   ├── casper_keys_service.rs   # Casper-specific implementation
│   ├── ethereum_keys_service.rs # Ethereum-specific implementation
│   ├── cosmos_keys_service.rs   # Cosmos-specific implementation
│   ├── crypto_service.rs        # Cryptographic operations
│   └── aws_kms_client_service.rs # AWS KMS integration
├── wasm_loader.rs       # WebAssembly module loader
└── tests/               # Test modules
```

### Adding New Blockchain Support

1. Implement the `KeysServiceTrait` for your blockchain
2. Add configuration options in `config.rs`
3. Update the service factory in `lib.rs`
4. Add tests for the new implementation
</details>

<details>
  <summary><strong><code>Deployment</code></strong></summary>

### Production Deployment

1. Set `TESTING_MODE=false`
2. Configure AWS credentials
3. Set appropriate `BLOCKCHAIN_MODE`
4. Use a reverse proxy (nginx, etc.)
5. Enable HTTPS/TLS

### Docker Deployment

```bash
# Build production image
docker build -t kms-secp256k1-api:latest .

# Run with environment variables
docker run -d \
  -p 4000:4000 \
  -e TESTING_MODE=false \
  -e AWS_MODE=true \
  -e AWS_REGION=us-west-2 \
  kms-secp256k1-api:latest
```

</details>

<details>
  <summary><strong><code>Monitoring and Logging</code></strong></summary>

The API uses structured logging with configurable levels:

```bash
# Set log level
export RUST_LOG=info

# Available levels: error, warn, info, debug, trace
```

</details>

<details>
  <summary><strong><code>Contributing</code></strong></summary>

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request
</details>

<details>
  <summary><strong><code>License</code></strong></summary>

This project is licensed under the MIT License - see below for details:

```
MIT License

Copyright (c) 2024 KMS secp256k1 API Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

</details>

<details>
  <summary><strong><code>Support</code></strong></summary>

For support and questions:

- Create an issue on GitHub
- Check the API documentation at `/api/`
- Review the test examples in the `tests/` directory
</details>

<details>
  <summary><strong><code>Related Projects</code></strong></summary>

- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [k256](https://github.com/RustCrypto/elliptic-curves) - secp256k1 implementation
- [AWS SDK for Rust](https://github.com/awslabs/aws-sdk-rust) - AWS integration
</details>

---

IMPORTANT WARNING

**This software is currently Work-In-Progress (WIP) and is provided for educational and development purposes only.**

**USE AT YOUR OWN RISK!**

- This software has not been audited for security vulnerabilities
- The authors are not responsible for any loss of keys, funds, or other assets
- This software may contain bugs or security flaws that could result in financial losses
- Do not use this software in production environments without thorough testing
- Always test with small amounts before using with significant assets
- Consider this software experimental until it reaches a stable release

**By using this software, you acknowledge that you understand these risks and agree to use it at your own risk.**
