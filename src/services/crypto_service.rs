use crate::{
    constants::DEFAULT_ETH_CHAIN_ID,
    wasm_loader::{WasmInstance, WasmLoader},
};
use std::{error::Error, sync::Arc};
use wasmtime::TypedFunc;

pub type CryptoService = WasmInstance;

impl CryptoService {
    /// Creates a new `CryptoService` by instantiating a WASM module from the given loader.
    ///
    /// # Errors
    ///
    /// Returns an error if the WASM module instantiation fails.
    pub fn new(loader: &Arc<WasmLoader>) -> Result<Self, Box<dyn Error>> {
        let instance = loader.instantiate()?;
        Ok(instance)
    }

    fn invoke_wasm_str_func(
        &mut self,
        func: &TypedFunc<(i32, i32), i32>,
        input: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let input_bytes = input.as_bytes();
        let input_len = i32::try_from(input_bytes.len()).unwrap_or_default();

        // 1. Allocation
        let alloc_func = self.alloc.typed::<i32, i32>(&self.store)?;
        let input_ptr_i32 = alloc_func.call(&mut self.store, input_len)?;
        if input_ptr_i32 < 0 {
            return Err("Allocation failed: negative pointer returned".into());
        }
        let input_ptr = usize::try_from(input_ptr_i32).map_err(|_| "Pointer conversion failed")?;

        // 2. Write input into memory
        let mem_data = self.memory.data_mut(&mut self.store);
        let input_len_usize = usize::try_from(input_len).map_err(|_| "Invalid input length")?;
        mem_data[input_ptr..input_ptr + input_len_usize].copy_from_slice(input_bytes);

        // 3. Call wasm function
        let output_ptr = func.call(
            &mut self.store,
            (i32::try_from(input_ptr).unwrap_or_default(), input_len),
        )?;
        if output_ptr == 0 {
            return Err("WASM function returned null pointer".into());
        }

        // 4. Read null-terminated result string
        let result_str = {
            let mem_data = self.memory.data(&self.store);
            let output_ptr_usize =
                usize::try_from(output_ptr).map_err(|_| "Invalid output pointer")?;

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
    /// Returns an error if the WASM function cannot be typed or invoked correctly.
    pub fn public_key(&mut self, public_key: &str) -> Result<String, Box<dyn std::error::Error>> {
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
    pub fn address_eth(&mut self, public_key: &str) -> Result<String, Box<dyn std::error::Error>> {
        let func = self.address_eth.typed::<(i32, i32), i32>(&self.store)?;
        self.invoke_wasm_str_func(&func, public_key)
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
    pub fn convert(&mut self, signature: &str) -> Result<String, Box<dyn std::error::Error>> {
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
    pub fn unconvert(&mut self, signature: &str) -> Result<String, Box<dyn std::error::Error>> {
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
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let alloc_func = self.alloc.typed::<i32, i32>(&self.store)?;
        let free_func = self.free.typed::<(i32, i32), ()>(&self.store)?;
        let func = self
            .verify
            .typed::<(i32, i32, i32, i32, i32, i32), i32>(&self.store)?;

        // Helper closure to allocate memory and write input string bytes
        let mut alloc_and_write = |input: &str| -> Result<(i32, i32), Box<dyn std::error::Error>> {
            let bytes = input.as_bytes();
            let len = i32::try_from(bytes.len()).unwrap_or_default();
            let ptr_i32 = alloc_func.call(&mut self.store, len)?;
            let ptr =
                usize::try_from(ptr_i32).map_err(|_| "Invalid pointer returned from alloc")?;
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
    ) -> Result<String, Box<dyn std::error::Error>> {
        let alloc_func = self.alloc.typed::<i32, i32>(&self.store)?;
        let free_func = self.free.typed::<(i32, i32), ()>(&self.store)?;
        let func = self
            .recover_v
            .typed::<(i32, i32, i32, i32, i32, i32, i32), i32>(&self.store)?;

        let mut alloc_and_write = |input: &str| -> Result<(i32, i32), Box<dyn std::error::Error>> {
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

        let chain_id = eth_chain_id.unwrap_or(DEFAULT_ETH_CHAIN_ID) as i32;

        let ret_ptr = func.call(
            &mut self.store,
            (msg_ptr, msg_len, sig_ptr, sig_len, pk_ptr, pk_len, chain_id),
        )?;

        free_func.call(&mut self.store, (msg_ptr, msg_len))?;
        free_func.call(&mut self.store, (sig_ptr, sig_len))?;
        free_func.call(&mut self.store, (pk_ptr, pk_len))?;

        if ret_ptr == 0 {
            return Err("recover_v returned null pointer".into());
        }

        let mem_data = self.memory.data(&self.store);
        let mut end = ret_ptr as usize;
        while end < mem_data.len() && mem_data[end] != 0 {
            end += 1;
        }
        let cstr_bytes = &mem_data[ret_ptr as usize..end];
        let result_str = std::str::from_utf8(cstr_bytes)?.to_string();

        free_func.call(&mut self.store, (ret_ptr, result_str.len() as i32))?;

        Ok(result_str)
    }
    /// Verifies an Ethereum EIP-155 message signature using a public key via the WASM module.
    ///
    /// # Arguments
    ///
    /// * `message_hash` - The 32-byte hex-encoded hash of the original message.
    /// * `signature` - The 65-byte signature (r[32] + s[32] + v[1]), hex-encoded.
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
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let alloc_func = self.alloc.typed::<i32, i32>(&self.store)?;
        let free_func = self.free.typed::<(i32, i32), ()>(&self.store)?;
        let func = self
            .verify_eip155
            .typed::<(i32, i32, i32, i32, i32, i32), i32>(&self.store)?;

        // Helper to allocate and write string input
        let mut alloc_and_write = |input: &str| -> Result<(i32, i32), Box<dyn std::error::Error>> {
            let bytes = input.as_bytes();
            let len = i32::try_from(bytes.len()).unwrap_or_default();
            let ptr_i32 = alloc_func.call(&mut self.store, len)?;
            let ptr =
                usize::try_from(ptr_i32).map_err(|_| "Invalid pointer returned from alloc")?;
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
            CASPER_PUBLIC_KEY_PREFIXED, CASPER_SECP_PREFIX, ETH_PUBLIC_KEY, ETH_SIGNATURE,
            ETH_SIGNATURE_V, ETH_TRANSACTION_HASH, SIGNATURE, SIGNATURE_PREFIXED, SIGNATURE_RS_LEN,
            TRANSACTION_HASH, WASM_PATH,
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
                // println!("Converted public key hex: {hex}");
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
}
