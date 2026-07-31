use crate::KmsError;
use crate::{
    constants::DEFAULT_ETH_CHAIN_ID,
    wasm_loader::{WasmInstance, WasmLoader},
};
use std::sync::Arc;
use wasmtime::TypedFunc;

pub type CryptoService = WasmInstance;

impl CryptoService {
    /// Creates a new `CryptoService` by instantiating a WASM module from the given loader.
    ///
    /// # Errors
    ///
    /// Returns [`KmsError::Crypto`] if the WASM module instantiation fails.
    pub fn new(loader: &Arc<WasmLoader>) -> crate::Result<Self> {
        let instance = loader.instantiate()?;
        Ok(instance)
    }

    fn invoke_wasm_str_func(
        &mut self,
        func: &TypedFunc<(i32, i32), i32>,
        input: &str,
    ) -> crate::Result<String> {
        let input_bytes = input.as_bytes();
        let input_len = i32::try_from(input_bytes.len()).unwrap_or_default();

        // 1. Allocation
        let alloc_func = self.alloc.typed::<i32, i32>(&self.store)?;
        let input_ptr_i32 = alloc_func.call(&mut self.store, input_len)?;
        if input_ptr_i32 < 0 {
            return Err(KmsError::Crypto(
                "Allocation failed: negative pointer returned".into(),
            ));
        }
        let input_ptr = usize::try_from(input_ptr_i32)
            .map_err(|_| KmsError::Crypto("Pointer conversion failed".into()))?;

        // 2. Write input into memory
        let mem_data = self.memory.data_mut(&mut self.store);
        let input_len_usize = usize::try_from(input_len)
            .map_err(|_| KmsError::Crypto("Invalid input length".into()))?;
        mem_data[input_ptr..input_ptr + input_len_usize].copy_from_slice(input_bytes);

        // 3. Call wasm function
        let output_ptr = func.call(
            &mut self.store,
            (i32::try_from(input_ptr).unwrap_or_default(), input_len),
        )?;
        if output_ptr == 0 {
            return Err(KmsError::Crypto(
                "WASM function returned null pointer".into(),
            ));
        }

        // 4. Read null-terminated result string
        let result_str = {
            let mem_data = self.memory.data(&self.store);
            let output_ptr_usize = usize::try_from(output_ptr)
                .map_err(|_| KmsError::Crypto("Invalid output pointer".into()))?;

            let mut end = output_ptr_usize;
            while end < mem_data.len() && mem_data[end] != 0 {
                end += 1;
            }
            let output_bytes = &mem_data[output_ptr_usize..end];
            std::str::from_utf8(output_bytes)?.to_string()
        };

        // 5. Free memory
        let free_func = self.free.typed::<(i32, i32), ()>(&self.store)?;
        free_func.call(
            &mut self.store,
            (
                output_ptr,
                i32::try_from(result_str.len() + 1).unwrap_or_default(),
            ),
        )?;
        free_func.call(
            &mut self.store,
            (i32::try_from(input_ptr).unwrap_or_default(), input_len),
        )?;

        Ok(result_str)
    }

    ///
    /// # Arguments
    ///
    /// * `public_key` - The public key as a string.
    ///
    /// # Returns
    ///
    /// A `Result` containing the hexadecimal string if successful, or an error if the WASM invocation fails.
    ///
    /// # Errors
    ///
    /// Returns [`KmsError::Crypto`] if the WASM function cannot be typed or invoked correctly.
    pub fn public_key(&mut self, public_key: &str) -> crate::Result<String> {
        let func = self.public_key.typed::<(i32, i32), i32>(&self.store)?;
        self.invoke_wasm_str_func(&func, public_key)
    }

    ///
    /// # Arguments
    ///
    /// * `public_key` - The public key as a string.
    ///
    /// # Returns
    ///
    /// A `Result` containing the Ethereum address as a hexadecimal string (with `0x` prefix) if successful,
    /// or an error if the WASM invocation fails.
    ///
    /// # Errors
    ///
    /// Returns an error if the WASM function cannot be typed or invoked correctly,
    /// or if the WASM function returns an error string.
    pub fn address_eth(&mut self, public_key: &str) -> crate::Result<String> {
        let func = self.address_eth.typed::<(i32, i32), i32>(&self.store)?;
        self.invoke_wasm_str_func(&func, public_key)
    }

    ///
    /// # Arguments
    ///
    /// * `public_key` - The compressed secp256k1 public key as a hex string (33 bytes, starts with 0x02 or 0x03).
    /// * `hrp` - The desired Bech32 HRP prefix (e.g., "cosmos", "osmo"). If empty or invalid, defaults to `"cosmos"`.
    ///
    /// # Returns
    ///
    /// A `Result` containing the Cosmos address as a Bech32m string if successful,
    /// or an error if the WASM invocation fails.
    ///
    /// # Errors
    ///
    /// Returns an error if the WASM function cannot be typed or invoked correctly,
    /// or if the WASM function returns an error string.
    pub fn address_cosmos(&mut self, public_key: &str, hrp: &str) -> crate::Result<String> {
        let alloc_func = self.alloc.typed::<i32, i32>(&self.store)?;
        let free_func = self.free.typed::<(i32, i32), ()>(&self.store)?;
        let func = self
            .address_cosmos
            .typed::<(i32, i32, i32, i32), i32>(&self.store)?;

        // Helper closure to allocate and write string input into WASM memory
        let mut alloc_and_write = |input: &str| -> crate::Result<(i32, i32)> {
            let bytes = input.as_bytes();
            let len = i32::try_from(bytes.len())?;
            let ptr_i32 = alloc_func.call(&mut self.store, len)?;
            let ptr = usize::try_from(ptr_i32)?;
            let mem = self.memory.data_mut(&mut self.store);
            mem[ptr..ptr + bytes.len()].copy_from_slice(bytes);
            Ok((ptr_i32, len))
        };

        // Write the public key and hrp into WASM memory
        let (pk_ptr, pk_len) = alloc_and_write(public_key)?;
        let (ud_ptr, ud_len) = alloc_and_write(hrp)?;

        // Call the WASM function
        let ret_ptr = func.call(&mut self.store, (pk_ptr, pk_len, ud_ptr, ud_len))?;

        // Free input memory
        free_func.call(&mut self.store, (pk_ptr, pk_len))?;
        free_func.call(&mut self.store, (ud_ptr, ud_len))?;

        // Check for null return
        if ret_ptr == 0 {
            return Err(KmsError::Crypto(
                "address_cosmos returned null pointer".into(),
            ));
        }

        // Read null-terminated string from WASM memory
        let mem = self.memory.data(&self.store);
        let mut end = usize::try_from(ret_ptr)
            .map_err(|_| KmsError::Crypto("address_cosmos pointer cannot be negative".into()))?;
        while end < mem.len() && mem[end] != 0 {
            end += 1;
        }
        let start = usize::try_from(ret_ptr)
            .map_err(|_| KmsError::Crypto("ret_ptr cannot be negative".into()))?;
        let result_bytes = &mem[start..end];
        let result_str = std::str::from_utf8(result_bytes)?.to_string();

        // Free result string in WASM memory
        let len_i32 = i32::try_from(result_str.len())
            .map_err(|_| KmsError::Crypto("result_str length exceeds i32::MAX".into()))?;
        free_func.call(&mut self.store, (ret_ptr, len_i32))?;

        Ok(result_str)
    }

    ///
    /// # Arguments
    ///
    /// * `signature` - The signature to convert as a string.
    ///
    /// # Returns
    ///
    /// A `Result` containing the converted string if successful, or an error if the WASM invocation fails.
    ///
    /// # Errors
    ///
    /// Returns an error if the WASM function cannot be typed or invoked correctly.
    pub fn convert(&mut self, signature: &str) -> crate::Result<String> {
        let func = self.convert.typed::<(i32, i32), i32>(&self.store)?;
        self.invoke_wasm_str_func(&func, signature)
    }

    ///
    /// # Arguments
    ///
    /// * `signature` - The converted signature as a string.
    ///
    /// # Returns
    ///
    /// A `Result` containing the original signature if successful, or an error if the WASM invocation fails.
    ///
    /// # Errors
    ///
    /// Returns an error if the WASM function cannot be typed or invoked correctly.
    pub fn unconvert(&mut self, signature: &str) -> crate::Result<String> {
        let func = self.unconvert.typed::<(i32, i32), i32>(&self.store)?;
        self.invoke_wasm_str_func(&func, signature)
    }

    /// Verifies a message signature using a public key via the WASM module.
    ///
    /// # Arguments
    ///
    /// * `message` - The original message.
    /// * `signature` - The signature to verify.
    /// * `public_key` - The public key used for verification.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the signature is valid, `Ok(false)` if invalid, or an error on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if memory allocation or WASM function invocation fails.
    pub fn verify(
        &mut self,
        message: &str,
        signature: &str,
        public_key: &str,
    ) -> crate::Result<bool> {
        let alloc_func = self.alloc.typed::<i32, i32>(&self.store)?;
        let free_func = self.free.typed::<(i32, i32), ()>(&self.store)?;
        let func = self
            .verify
            .typed::<(i32, i32, i32, i32, i32, i32), i32>(&self.store)?;

        // Helper closure to allocate memory and write input string bytes
        let mut alloc_and_write = |input: &str| -> crate::Result<(i32, i32)> {
            let bytes = input.as_bytes();
            let len = i32::try_from(bytes.len()).unwrap_or_default();
            let ptr_i32 = alloc_func.call(&mut self.store, len)?;
            let ptr = usize::try_from(ptr_i32)
                .map_err(|_| KmsError::Crypto("Invalid pointer returned from alloc".into()))?;
            let mem_data = self.memory.data_mut(&mut self.store);
            mem_data[ptr..ptr + bytes.len()].copy_from_slice(bytes);
            Ok((ptr_i32, len))
        };

        // Allocate and write each input string
        let (msg_ptr, msg_len) = alloc_and_write(message)?;
        let (sig_ptr, sig_len) = alloc_and_write(signature)?;
        let (pk_ptr, pk_len) = alloc_and_write(public_key)?;

        // Call the WASM verify function with pointers and lengths
        let result = func.call(
            &mut self.store,
            (msg_ptr, msg_len, sig_ptr, sig_len, pk_ptr, pk_len),
        )?;

        // Free allocated memory after call
        free_func.call(&mut self.store, (msg_ptr, msg_len))?;
        free_func.call(&mut self.store, (sig_ptr, sig_len))?;
        free_func.call(&mut self.store, (pk_ptr, pk_len))?;

        Ok(result == 1)
    }

    /// Recovers the Ethereum-compatible recovery ID `v` (as a hex string) from a signature and public key via the WASM module.
    ///
    /// # Arguments
    ///
    /// * `message` - The original message hash as a hex string (64 characters).
    /// * `signature` - The signature without the recovery ID (r + s) as a hex string (128 characters).
    /// * `public_key` - The compressed public key as a hex string (66 characters).
    ///
    /// # Returns
    ///
    /// Returns `Ok(String)` containing the recovered `v` value in hex (e.g., `"1b"`) if successful,
    /// or an error string from the WASM module if recovery fails.
    ///
    /// # Errors
    ///
    /// Returns an error if memory allocation fails, the WASM function invocation fails,
    /// or if the returned pointer is null or the returned string is invalid UTF-8.
    pub fn recover_v(
        &mut self,
        message: &str,
        signature: &str,
        public_key: &str,
        eth_chain_id: Option<u8>,
    ) -> crate::Result<String> {
        let alloc_func = self.alloc.typed::<i32, i32>(&self.store)?;
        let free_func = self.free.typed::<(i32, i32), ()>(&self.store)?;
        let func = self
            .recover_v
            .typed::<(i32, i32, i32, i32, i32, i32, i32), i32>(&self.store)?;

        let mut alloc_and_write = |input: &str| -> crate::Result<(i32, i32)> {
            let bytes = input.as_bytes();
            let len = i32::try_from(bytes.len())?;
            let ptr_i32 = alloc_func.call(&mut self.store, len)?;
            let ptr = usize::try_from(ptr_i32)?;
            let mem_data = self.memory.data_mut(&mut self.store);
            mem_data[ptr..ptr + bytes.len()].copy_from_slice(bytes);
            Ok((ptr_i32, len))
        };

        let (msg_ptr, msg_len) = alloc_and_write(message)?;
        let (sig_ptr, sig_len) = alloc_and_write(signature)?;
        let (pk_ptr, pk_len) = alloc_and_write(public_key)?;

        let chain_id = i32::from(eth_chain_id.unwrap_or(DEFAULT_ETH_CHAIN_ID));

        let ret_ptr = func.call(
            &mut self.store,
            (msg_ptr, msg_len, sig_ptr, sig_len, pk_ptr, pk_len, chain_id),
        )?;

        free_func.call(&mut self.store, (msg_ptr, msg_len))?;
        free_func.call(&mut self.store, (sig_ptr, sig_len))?;
        free_func.call(&mut self.store, (pk_ptr, pk_len))?;

        if ret_ptr == 0 {
            return Err(KmsError::Crypto("recover_v returned null pointer".into()));
        }

        let mem_data = self.memory.data(&self.store);

        let mut end = usize::try_from(ret_ptr)
            .map_err(|_| KmsError::Crypto("ret_ptr cannot be negative".into()))?;
        while end < mem_data.len() && mem_data[end] != 0 {
            end += 1;
        }
        let start = usize::try_from(ret_ptr)
            .map_err(|_| KmsError::Crypto("ret_ptr cannot be negative".into()))?;
        let cstr_bytes = &mem_data[start..end];
        let result_str = std::str::from_utf8(cstr_bytes)?.to_string();

        let len_i32 = i32::try_from(result_str.len())
            .map_err(|_| KmsError::Crypto("result_str length exceeds i32::MAX".into()))?;
        free_func.call(&mut self.store, (ret_ptr, len_i32))?;

        Ok(result_str)
    }
    /// Verifies an Ethereum EIP-155 message signature using a public key via the WASM module.
    ///
    /// # Arguments
    ///
    /// * `message_hash` - The 32-byte hex-encoded hash of the original message.
    /// * `signature` - The 65-byte signature (r\[32\] + s\[32\] + v\[1\]), hex-encoded.
    /// * `public_key` - The compressed 33-byte public key, hex-encoded.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the signature is valid, `Ok(false)` if invalid, or an error on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if memory allocation or WASM function invocation fails.
    pub fn verify_eip155(
        &mut self,
        message_hash: &str,
        signature: &str,
        public_key: &str,
    ) -> crate::Result<bool> {
        let alloc_func = self.alloc.typed::<i32, i32>(&self.store)?;
        let free_func = self.free.typed::<(i32, i32), ()>(&self.store)?;
        let func = self
            .verify_eip155
            .typed::<(i32, i32, i32, i32, i32, i32), i32>(&self.store)?;

        // Helper to allocate and write string input
        let mut alloc_and_write = |input: &str| -> crate::Result<(i32, i32)> {
            let bytes = input.as_bytes();
            let len = i32::try_from(bytes.len()).unwrap_or_default();
            let ptr_i32 = alloc_func.call(&mut self.store, len)?;
            let ptr = usize::try_from(ptr_i32)
                .map_err(|_| KmsError::Crypto("Invalid pointer returned from alloc".into()))?;
            let mem_data = self.memory.data_mut(&mut self.store);
            mem_data[ptr..ptr + bytes.len()].copy_from_slice(bytes);
            Ok((ptr_i32, len))
        };

        // Allocate and write message, signature, and public key
        let (msg_ptr, msg_len) = alloc_and_write(message_hash)?;
        let (sig_ptr, sig_len) = alloc_and_write(signature)?;
        let (pk_ptr, pk_len) = alloc_and_write(public_key)?;

        // Invoke WASM function
        let result = func.call(
            &mut self.store,
            (msg_ptr, msg_len, sig_ptr, sig_len, pk_ptr, pk_len),
        )?;

        // Free allocated memory
        free_func.call(&mut self.store, (msg_ptr, msg_len))?;
        free_func.call(&mut self.store, (sig_ptr, sig_len))?;
        free_func.call(&mut self.store, (pk_ptr, pk_len))?;

        Ok(result == 1)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        constants::{
            CASPER_PUBLIC_KEY_PREFIXED, CASPER_SECP_PREFIX, COSMOS_PUBLIC_KEY, DEFAULT_COSMOS_HRP,
            ETH_PUBLIC_KEY, ETH_SIGNATURE, ETH_SIGNATURE_V, ETH_TRANSACTION_HASH, SIGNATURE,
            SIGNATURE_PREFIXED, SIGNATURE_RS_LEN, TRANSACTION_HASH, WASM_PATH,
        },
        services::crypto_service::CryptoService,
        wasm_loader::WasmLoader,
    };

    #[tokio::test]
    async fn test_crypto_service_public_key() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let public_key_input = "MFYwEAYHKoZIzj0CAQYFK4EEAAoDQgAE95tY6siYB+8oxfUTxjuBxNd+LEqeXrLT4TZ7jSSvoGlk+5z1txG+n4Zmeii1ncqyMqaghf//enmalrKJdF8mZQ==";

        let result = crypto_service.public_key(public_key_input);

        match result {
            Ok(hex) => {
                assert!(!hex.is_empty(), "Expected non-empty hex output");
            }
            Err(e) => panic!("Failed to convert public key: {e}"),
        }
    }

    #[tokio::test]
    async fn test_crypto_service_verify() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Full key with prefix
        let message = TRANSACTION_HASH;
        let signature = SIGNATURE_PREFIXED;
        let public_key = CASPER_PUBLIC_KEY_PREFIXED;

        let is_valid = crypto_service
            .verify(message, signature, public_key)
            .expect("Verification failed");

        assert!(
            is_valid,
            "Expected valid signature for public key with prefix"
        );

        // Key without prefix
        let signature = SIGNATURE;
        let public_key = CASPER_PUBLIC_KEY_PREFIXED.trim_start_matches(CASPER_SECP_PREFIX);

        let is_valid = crypto_service
            .verify(message, signature, public_key)
            .expect("Verification failed");

        assert!(
            is_valid,
            "Expected valid signature for compressed public key format"
        );
    }

    #[tokio::test]
    async fn test_crypto_service_verify_eip155() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Full key with prefix
        let message = ETH_TRANSACTION_HASH;
        let signature = ETH_SIGNATURE;
        let public_key = ETH_PUBLIC_KEY;

        let is_valid = crypto_service
            .verify_eip155(message, signature, public_key)
            .expect("Verification failed");

        assert!(is_valid, "Expected valid signature for public key");
    }

    #[tokio::test]
    async fn test_crypto_service_unconvert() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Signature with prefix
        let signature_with_prefix = SIGNATURE_PREFIXED;
        let result_with_prefix = crypto_service
            .unconvert(signature_with_prefix)
            .expect("Failed to unconvert signature with prefix");

        assert!(
            !result_with_prefix.is_empty(),
            "Expected non-empty result for signature with prefix"
        );

        // Signature without prefix
        let signature_without_prefix = SIGNATURE;
        let result_without_prefix = crypto_service
            .unconvert(signature_without_prefix)
            .expect("Failed to unconvert plain signature");

        assert!(
            !result_without_prefix.is_empty(),
            "Expected non-empty result for plain signature"
        );

        assert_eq!(result_with_prefix, result_without_prefix);
    }

    #[tokio::test]
    async fn test_crypto_service_unconvert_eip155() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let signature = ETH_SIGNATURE.to_string();
        let result = crypto_service
            .unconvert(&signature)
            .expect("Failed to unconvert signature");

        assert!(
            !result.is_empty(),
            "Expected non-empty result for signature with prefix"
        );

        assert_eq!(result, result);
    }

    #[tokio::test]
    async fn test_crypto_service_convert_roundtrip() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let signature_without_prefix = SIGNATURE;

        let signature = crypto_service
            .unconvert(signature_without_prefix)
            .expect("Failed to unconvert signature with prefix");

        let signature = crypto_service
            .convert(&signature)
            .expect("Failed to convert back to signature");

        assert_eq!(
            signature.to_lowercase(),
            signature_without_prefix.to_lowercase()
        );
    }

    #[tokio::test]
    async fn test_crypto_service_case_insensitive_signature_roundtrip() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let signature = SIGNATURE.to_uppercase();

        let unconverted = crypto_service.unconvert(&signature).unwrap();
        let result = crypto_service.convert(&unconverted).unwrap();

        assert_eq!(result.to_lowercase(), SIGNATURE.to_lowercase());
    }

    #[tokio::test]
    async fn test_crypto_service_convert_roundtrip_eip155() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let signature_without_prefix = ETH_SIGNATURE;

        let signature = crypto_service
            .unconvert(signature_without_prefix)
            .expect("Failed to unconvert signature");

        let signature = crypto_service
            .convert(&signature)
            .expect("Failed to convert back to signature");

        assert_eq!(
            signature.to_lowercase(),
            signature_without_prefix.to_lowercase()
        );
    }

    #[tokio::test]
    async fn test_crypto_service_convert_with_prefix_roundtrip() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let signature_with_prefix = SIGNATURE_PREFIXED;

        let signature = crypto_service
            .unconvert(signature_with_prefix)
            .expect("Failed to unconvert signature with prefix");

        let signature = crypto_service
            .convert(&signature)
            .expect("Failed to convert back to signature");

        assert_eq!(
            format!("{CASPER_SECP_PREFIX}{}", signature.to_lowercase()),
            signature_with_prefix.to_lowercase()
        );
    }

    #[tokio::test]
    async fn test_crypto_service_recover_v() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let message = TRANSACTION_HASH;
        let signature = &SIGNATURE;
        let public_key = CASPER_PUBLIC_KEY_PREFIXED;

        let result = crypto_service
            .recover_v(message, signature, public_key, None)
            .expect("Failed to recover v");

        assert!(!result.is_empty(), "Expected non-empty v from recover_v");

        assert_eq!(result, "0", "Expected non-empty 0 v");
    }

    #[tokio::test]
    async fn test_crypto_service_recover_v_eip155() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let message = ETH_TRANSACTION_HASH;
        let signature = &ETH_SIGNATURE[..SIGNATURE_RS_LEN]; // remove recovery byte (r + s)
        let public_key = ETH_PUBLIC_KEY;

        let result = crypto_service
            .recover_v(message, signature, public_key, None)
            .expect("Failed to recover v");

        assert_eq!(result, ETH_SIGNATURE_V, "Expected non-empty v");
    }

    #[tokio::test]
    async fn test_crypto_service_recover_v_with_chain_id() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let message = ETH_TRANSACTION_HASH;
        let signature = &ETH_SIGNATURE[..SIGNATURE_RS_LEN];
        let public_key = ETH_PUBLIC_KEY;

        let chain_id = Some(1); // Ethereum mainnet

        let result = crypto_service
            .recover_v(message, signature, public_key, chain_id)
            .expect("Failed to recover v");

        assert_eq!(result, ETH_SIGNATURE_V, "Expected non-empty v");

        let result = crypto_service
            .recover_v(message, signature, public_key, None)
            .expect("Failed to recover v");

        assert_eq!(result, ETH_SIGNATURE_V, "Expected non-empty v");
    }

    #[tokio::test]
    async fn test_crypto_service_invalid_public_key() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let invalid_key = "invalid_key";

        let result = crypto_service.public_key(invalid_key);

        assert!(
            result.is_err(),
            "Expected an error when passing invalid public key input"
        );
    }

    #[tokio::test]
    async fn test_crypto_service_empty_signature() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let message = ETH_TRANSACTION_HASH;
        let signature = "";
        let public_key = ETH_PUBLIC_KEY;

        let result = crypto_service.verify_eip155(message, signature, public_key);

        assert!(
            result.is_err() || !result.unwrap(),
            "Expected false or error for empty signature"
        );
    }

    #[tokio::test]
    async fn test_crypto_service_address_eth() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let public_key = ETH_PUBLIC_KEY;

        let address = crypto_service
            .address_eth(public_key)
            .expect("Failed to derive Ethereum address");

        assert!(
            address.starts_with("0x") && address.len() == 42,
            "Expected valid Ethereum address, got: {address}"
        );
    }

    #[tokio::test]
    async fn test_crypto_service_address_cosmos() {
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let mut crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let public_key = COSMOS_PUBLIC_KEY;

        let address = crypto_service
            .address_cosmos(public_key, DEFAULT_COSMOS_HRP)
            .expect("Failed to derive Cosmos address");

        // Use the constant for prefix dynamically
        let expected_prefix = format!("{DEFAULT_COSMOS_HRP}1");

        assert!(
            address.starts_with(&expected_prefix),
            "Address should start with '{expected_prefix}', got: {address}"
        );

        assert!(
            address.len() >= expected_prefix.len() + 38,
            "Address length too short: {}",
            address.len()
        );
    }
}
