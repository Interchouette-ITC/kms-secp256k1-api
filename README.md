# KMS secp256k1 API

A high-performance **custodial software wallet** built in Rust that serves as middleware between your blockchain applications and secure key storage on KMS providers. This API provides cryptographic operations for multiple blockchain networks using secp256k1 elliptic curve cryptography while keeping private keys secure in cloud-based key management systems.

Native support for Casper, Ethereum, or Cosmos networks

## 🏦 What This Custodial KMS API Does

### **Blockchain Perspective**
This software acts as a **custodial wallet service** that:
- **Generates cryptographic keypairs** for blockchain operations
- **Signs transactions, deploys, and messages** without exposing private keys
- **Manages key lifecycle** (creation, deletion, listing) through secure KMS providers
- **Supports multiple blockchain networks** (Casper, Ethereum, Cosmos) with their specific cryptographic requirements

### **Middleware Architecture**
The API serves as a **secure bridge** between your applications and cloud-based key storage:
- **Application Layer**: Your blockchain apps make HTTP requests to sign transactions
- **API Layer**: This service handles the cryptographic operations
- **KMS Layer**: Private keys are securely stored in AWS KMS (currently supporting secp256k1)
- **Blockchain Layer**: Signed transactions are returned to your application

### **Key Security Benefits**
- **Private keys never leave the KMS**: All signing operations happen within AWS KMS
- **No local key storage**: Eliminates risk of local key compromise
- **Audit trails**: All key operations are logged and traceable
- **Access control**: Fine-grained permissions for different operations (sign, create, delete, list)
- **Hardware security**: Leverages AWS KMS hardware security modules (HSMs)

### **How It Works**
1. **Key Generation**: Creates keypairs in AWS KMS with secp256k1 curve
2. **Signing Process**: 
   - Your app sends transaction hash/message to the API
   - API requests AWS KMS to sign using the stored private key
   - AWS KMS performs the cryptographic operation internally
   - Signed result is returned to your application
3. **Key Management**: Keys can be listed, deleted, and managed through the API

### **Use Cases**
- **DeFi Applications**: Secure transaction signing for decentralized finance
- **NFT Marketplaces**: Safe key management for digital asset transactions
- **Enterprise Blockchain**: Corporate blockchain solutions requiring key custody
- **Multi-chain Applications**: Single API for multiple blockchain networks
- **Compliance Requirements**: Meeting regulatory requirements for key custody

### **Route Security Implementation**

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

### **🔐 Best Practices for Key Management**

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

### **🚨 CRITICAL NETWORK SECURITY WARNING**

#### **⚠️ NEVER EXPOSE THIS API ON PUBLIC NETWORKS**

**This API software was designed for PRIVATE INFRASTRUCTURE ONLY and should NEVER be exposed to the public internet.**

#### **Why Public Exposure is Extremely Dangerous**
- **Anyone can sign with known public keys**: If an attacker discovers a public key, they can attempt to sign transactions
- **No built-in authentication**: The API has no authentication layer by default
- **Direct access to KMS operations**: Attackers can potentially access your AWS KMS keys
- **Financial risk**: Unauthorized transactions can result in loss of funds/assets
- **Compliance violations**: Public exposure may violate security and regulatory requirements

#### **🔐 MANDATORY AUTHENTICATION REQUIREMENTS**

**You SHALL/MUST implement an authentication guard before deploying this API on publci interfaces, any among those follwing are recommended:**

- **JWT Authentication**: Implement JWT token validation
- **API Key Authentication**: Use secure API keys with proper rotation
- **OAuth2/OIDC**: Enterprise-grade authentication systems
- **Network-level authentication**: VPN, IP whitelisting, or reverse proxy authentication
- **Rate limiting**: Prevent brute force attacks
- **Request signing**: HMAC-based request validation

#### **🌐 NETWORK DEPLOYMENT RECOMMENDATIONS**

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

## 🚀 Features

- **Multi-Blockchain Support**: Native support for Casper, Ethereum, and Cosmos networks
- **AWS KMS Integration**: Secure key management through AWS Key Management Service
- **RESTful API**: Clean HTTP endpoints with OpenAPI/Swagger documentation
- **High Performance**: Built with Axum web framework and async/await
- **WASM Support**: WebAssembly integration for cross-platform compatibility (cryptographic operations)
- **Testing Mode**: Mock services for development and testing
- **Comprehensive Logging**: Structured logging with configurable levels

## 🏗️ Architecture

The API is built with a modular architecture:

- **Routes Layer**: HTTP endpoints and request/response handling
- **Services Layer**: Business logic for different blockchain networks
- **Crypto Layer**: Core cryptographic operations using secp256k1
- **AWS Integration**: Secure key storage and management
- **WASM Loader**: Cross-platform cryptographic operations

## 🔐 WASM Cryptographic Operations

This API includes WebAssembly (WASM) integration for enhanced cryptographic operations. The WASM module is loaded from the `WASM_PATH` and provides additional cryptographic functionality beyond the native Rust implementations.

**Important Note**: The WASM binary file (`./wasm/wasm.wasm`) IS included in the repository, but the **source code that generates this WASM file is NOT open source** and is **NOT publicly available**. The proprietary source code contains specialized cryptographic algorithms and implementations that are not shared.

### WASM Features
- Enhanced cryptographic operations
- Cross-platform compatibility
- Optimized performance for specific algorithms
- Proprietary cryptographic implementations

## 📋 Prerequisites

- Rust 1.70+ with Cargo
- Docker (optional, for containerized deployment)
- AWS credentials (for production use)

## 🛠️ Installation

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

## ⚙️ Configuration

The API can be configured using environment variables. You can find an example configuration file in `.env.test` for testing purposes.

### 🔒 **Critical Security Configuration**

**⚠️ IMPORTANT: The following configuration options have significant security implications and should be carefully considered before enabling in production environments.**

### Core Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `APP_PORT` | `4000` | Server port |
| `TESTING_MODE` | `true` | Enable testing mode with mock services |
| `BLOCKCHAIN_MODE` | `casper` | Blockchain network (casper, ethereum, cosmos) |

### 🔐 **Security-Critical Configuration**

| Variable | Default | Description | **Security Risk** |
|----------|---------|-------------|-------------------|
| `DELETE_MODE` | `false` | Enable key deletion endpoints | **HIGH** - Can permanently delete keys |
| `LIST_MODE` | `false` | Enable key listing endpoints | **HIGH** - Exposes all available keys |

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

| Variable | Description |
|----------|-------------|
| `AWS_MODE` | Enable AWS KMS integration |
| `AWS_REGION` | AWS region for KMS operations |
| `KMS_SIGN_ID` | AWS access key for signing operations |
| `KMS_SIGN_KEY` | AWS secret key for signing operations |
| `KMS_CREATE_ID` | AWS access key for key creation |
| `KMS_CREATE_KEY` | AWS secret key for key creation |
| `KMS_DELETE_ID` | AWS access key for key deletion (optional) |
| `KMS_DELETE_KEY` | AWS secret key for key deletion (optional) |
| `KMS_LIST_ID` | AWS access key for key listing (optional) |
| `KMS_LIST_KEY` | AWS secret key for key listing (optional) |

### Blockchain-Specific Configuration

#### Ethereum
| Variable | Default | Description |
|----------|---------|-------------|
| `ETH_CHAIN_ID` | `1` | Ethereum chain ID |

#### Cosmos
| Variable | Default | Description |
|----------|---------|-------------|
| `COSMOS_CHAIN_ID` | `cosmoshub-4` | Cosmos chain ID |
| `COSMOS_HRP` | `cosmos` | Cosmos human-readable prefix |
| `COSMOS_REST_URL` | `https://rest.cosmos.network` | Cosmos REST endpoint |

## 🚀 Quick Start

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

## 📚 API Endpoints

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

## 🔐 Authentication

The API supports multiple authentication modes:

- **Testing Mode**: No authentication required (default)
- **AWS Mode**: AWS credentials for KMS operations
- **Production Mode**: Full AWS KMS integration

## 📖 Usage Examples

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

## 🧪 Testing

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

## 🔧 Development

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

## 🚀 Deployment

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

## 📊 Monitoring and Logging

The API uses structured logging with configurable levels:

```bash
# Set log level
export RUST_LOG=info

# Available levels: error, warn, info, debug, trace
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## 📄 License

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

## 🆘 Support

For support and questions:
- Create an issue on GitHub
- Check the API documentation at `/swagger-ui/`
- Review the test examples in the `tests/` directory

## 🔗 Related Projects

- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [k256](https://github.com/RustCrypto/elliptic-curves) - secp256k1 implementation
- [AWS SDK for Rust](https://github.com/awslabs/aws-sdk-rust) - AWS integration

---

## ⚠️ **IMPORTANT WARNING**

**This software is currently Work-In-Progress (WIP) and is provided for educational and development purposes only.**

**USE AT YOUR OWN RISK!**

- This software has not been audited for security vulnerabilities
- The authors are not responsible for any loss of keys, funds, or other assets
- This software may contain bugs or security flaws that could result in financial losses
- Do not use this software in production environments without thorough testing
- Always test with small amounts before using with significant assets
- Consider this software experimental until it reaches a stable release

**By using this software, you acknowledge that you understand these risks and agree to use it at your own risk.**
