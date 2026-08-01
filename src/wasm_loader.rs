use crate::KmsError;
use crate::constants::WASM_PATH;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::info;
use wasmtime::{Engine, Func, Instance, Memory, Module, Store};

/// WASM module bytes baked into the binary at compile time (Release asset needs no sidecar file).
const EMBEDDED_WASM: &[u8] = include_bytes!("../wasm/wasm.wasm");

pub struct WasmLoader {
    engine: Engine,
    module: Module,
}

pub struct WasmInstance {
    pub memory: Memory,
    pub alloc: Func,
    pub free: Func,
    pub public_key: Func,
    pub convert: Func,
    pub unconvert: Func,
    pub verify: Func,
    pub verify_eip155: Func,
    pub recover_v: Func,
    pub address_eth: Func,
    pub address_cosmos: Func,
    pub store: Store<()>,
    pub instance: Instance,
}

static WASM_INSTANCE: OnceCell<Arc<WasmLoader>> = OnceCell::const_new();

impl WasmLoader {
    /// Loads the WASM module (singleton).
    ///
    /// Resolution order:
    /// 1. `WASM_PATH` environment variable (file)
    /// 2. Default [`WASM_PATH`] file if it exists on disk
    /// 3. Bytes embedded via `include_bytes!`
    ///
    /// # Errors
    ///
    /// Returns [`KmsError::Crypto`] if the module cannot be compiled from the chosen source.
    pub async fn new() -> crate::Result<Arc<Self>> {
        WASM_INSTANCE
            .get_or_try_init(|| async move {
                let path = std::env::var("WASM_PATH").unwrap_or_else(|_| WASM_PATH.to_string());
                if Path::new(&path).is_file() {
                    Self::from_path(&path)
                } else {
                    Self::from_bytes(EMBEDDED_WASM, "embedded")
                }
            })
            .await
            .map(Arc::clone)
    }

    /// Load a WASM module from a filesystem path.
    ///
    /// # Errors
    ///
    /// Returns [`KmsError::Crypto`] if the file cannot be read or compiled.
    pub fn from_path(wasm_path: &str) -> crate::Result<Arc<Self>> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, wasm_path)
            .map_err(|e| KmsError::Crypto(format!("Failed to load WASM from {wasm_path}: {e}")))?;
        info!(source = %wasm_path, "WASM module loaded from file");
        Ok(Arc::new(Self { engine, module }))
    }

    /// Load a WASM module from raw bytes (typically the compile-time embed).
    ///
    /// # Errors
    ///
    /// Returns [`KmsError::Crypto`] if the bytes cannot be compiled.
    pub fn from_bytes(bytes: &[u8], source_label: &str) -> crate::Result<Arc<Self>> {
        let engine = Engine::default();
        let module = Module::new(&engine, bytes).map_err(|e| {
            KmsError::Crypto(format!("Failed to load WASM from {source_label}: {e}"))
        })?;
        info!(source = %source_label, "WASM module loaded from bytes");
        Ok(Arc::new(Self { engine, module }))
    }

    /// Instantiates the WASM module, initializing memory and required exported functions.
    ///
    /// # Errors
    ///
    /// Returns [`KmsError::Crypto`] if instance creation fails or a required export is missing.
    pub fn instantiate(&self) -> crate::Result<WasmInstance> {
        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &self.module, &[])?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| KmsError::Crypto("failed to find memory export".into()))?;

        let alloc = instance
            .get_func(&mut store, "alloc")
            .ok_or_else(|| KmsError::Crypto("failed to find alloc export".into()))?;

        let free = instance
            .get_func(&mut store, "free")
            .ok_or_else(|| KmsError::Crypto("failed to find free export".into()))?;

        let public_key = instance
            .get_func(&mut store, "public_key")
            .ok_or_else(|| KmsError::Crypto("failed to find public_key export".into()))?;

        let verify = instance
            .get_func(&mut store, "verify")
            .ok_or_else(|| KmsError::Crypto("failed to find verify export".into()))?;

        let verify_eip155 = instance
            .get_func(&mut store, "verify_eip155")
            .ok_or_else(|| KmsError::Crypto("failed to find verify_eip155 export".into()))?;

        let convert = instance
            .get_func(&mut store, "convert")
            .ok_or_else(|| KmsError::Crypto("failed to find convert export".into()))?;

        let unconvert = instance
            .get_func(&mut store, "unconvert")
            .ok_or_else(|| KmsError::Crypto("failed to find unconvert export".into()))?;

        let recover_v = instance
            .get_func(&mut store, "recover_v")
            .ok_or_else(|| KmsError::Crypto("failed to find recover_v export".into()))?;

        let address_eth = instance
            .get_func(&mut store, "address_eth")
            .ok_or_else(|| KmsError::Crypto("failed to find address_eth export".into()))?;

        let address_cosmos = instance
            .get_func(&mut store, "address_cosmos")
            .ok_or_else(|| KmsError::Crypto("failed to find address_cosmos export".into()))?;

        Ok(WasmInstance {
            memory,
            alloc,
            free,
            public_key,
            convert,
            unconvert,
            verify,
            verify_eip155,
            recover_v,
            address_eth,
            address_cosmos,
            store,
            instance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wasm_loader_singleton_loads_module() {
        let loader1 = WasmLoader::new().await.unwrap();
        let loader2 = WasmLoader::new().await.unwrap();

        assert!(Arc::ptr_eq(&loader1, &loader2));

        let module_bytes = loader1.module.serialize().unwrap();
        assert!(!module_bytes.is_empty(), "Module bytes should not be empty");
    }

    #[tokio::test]
    async fn test_instantiate_wasm_instance() {
        let loader = WasmLoader::new().await.expect("Failed to load wasm module");

        let wasm_instance = loader
            .instantiate()
            .expect("Failed to instantiate wasm instance");

        assert!(
            wasm_instance.memory.size(&wasm_instance.store) > 0,
            "Memory size should be > 0"
        );

        let alloc_ty = wasm_instance.alloc.ty(&wasm_instance.store);
        assert_eq!(alloc_ty.params().len(), 1, "alloc should take 1 parameter");
        assert_eq!(alloc_ty.results().len(), 1, "alloc should return 1 result");

        let free_ty = wasm_instance.free.ty(&wasm_instance.store);
        assert_eq!(free_ty.params().len(), 2, "free should take 2 parameters");
        assert_eq!(free_ty.results().len(), 0, "free should return no results");

        let public_key_ty = wasm_instance.public_key.ty(&wasm_instance.store);
        assert!(
            public_key_ty.params().len() > 0,
            "public_key should have at least one parameter"
        );

        let verify_ty = wasm_instance.verify.ty(&wasm_instance.store);
        assert!(
            verify_ty.params().len() > 0,
            "verify should have at least one parameter"
        );

        let convert_ty = wasm_instance.convert.ty(&wasm_instance.store);
        assert!(
            convert_ty.params().len() > 0,
            "convert should have at least one parameter"
        );

        let unconvert_ty = wasm_instance.unconvert.ty(&wasm_instance.store);
        assert!(
            unconvert_ty.params().len() > 0,
            "unconvert should have at least one parameter"
        );

        let recover_v_ty = wasm_instance.recover_v.ty(&wasm_instance.store);
        assert!(
            recover_v_ty.params().len() > 0,
            "recover_v should have at least one parameter"
        );

        let address_eth_ty = wasm_instance.address_eth.ty(&wasm_instance.store);
        assert!(
            address_eth_ty.params().len() > 0,
            "address_eth should have at least one parameter"
        );

        let address_cosmos_ty = wasm_instance.address_cosmos.ty(&wasm_instance.store);
        assert!(
            address_cosmos_ty.params().len() > 0,
            "address_cosmos should have at least one parameter"
        );
    }

    #[tokio::test]
    async fn test_from_bytes_embedded() {
        let loader = WasmLoader::from_bytes(EMBEDDED_WASM, "embedded-test")
            .expect("embedded WASM must compile");
        loader
            .instantiate()
            .expect("embedded WASM must instantiate");
    }
}
