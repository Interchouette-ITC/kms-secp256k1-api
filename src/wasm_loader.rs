use std::{error::Error, sync::Arc};
use tokio::sync::OnceCell;
use tracing::info;
use wasmtime::{Engine, Func, Instance, Memory, Module, Store};

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
    pub store: Store<()>,
    pub instance: Instance,
}

static WASM_INSTANCE: OnceCell<Arc<WasmLoader>> = OnceCell::const_new();

impl WasmLoader {
    /// Loads a WASM module from the specified file path and initializes a singleton instance.
    ///
    /// Returns an `Arc` pointing to the initialized `Self`.
    ///
    /// # Errors
    ///
    /// Returns an error if the WASM module fails to load from the given path.
    pub async fn new(wasm_path: &str) -> Result<Arc<Self>, Box<dyn Error>> {
        WASM_INSTANCE
            .get_or_try_init(|| async move {
                let engine = Engine::default();
                let module = Module::from_file(&engine, wasm_path)?;
                info!("WASM module loaded from {wasm_path}");
                Ok(Arc::new(Self { engine, module }))
            })
            .await
            .map(Arc::clone)
    }

    /// Instantiates the WASM module, initializing memory and required exported functions.
    ///
    /// Returns a `WasmInstance` containing references to the module’s memory and exported functions.
    ///
    /// # Errors
    ///
    /// Returns an error if the instance creation fails or if any of the required exports
    /// (memory, alloc, free, `public_key`, verify, convert, unconvert) cannot be found.
    pub fn instantiate(&self) -> Result<WasmInstance, Box<dyn Error>> {
        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &self.module, &[])?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or("failed to find memory export")?;

        let alloc = instance
            .get_func(&mut store, "alloc")
            .ok_or("failed to find alloc export")?;

        let free = instance
            .get_func(&mut store, "free")
            .ok_or("failed to find free export")?;

        let public_key = instance
            .get_func(&mut store, "public_key")
            .ok_or("failed to find public_key export")?;

        let verify = instance
            .get_func(&mut store, "verify")
            .ok_or("failed to find verify export")?;

        let verify_eip155 = instance
            .get_func(&mut store, "verify_eip155")
            .ok_or("failed to find verify_eip155 export")?;

        let convert = instance
            .get_func(&mut store, "convert")
            .ok_or("failed to find convert export")?;

        let unconvert = instance
            .get_func(&mut store, "unconvert")
            .ok_or("failed to find unconvert export")?;

        let recover_v = instance
            .get_func(&mut store, "recover_v")
            .ok_or("failed to find recover_v export")?;

        let address_eth = instance
            .get_func(&mut store, "address_eth")
            .ok_or("failed to find address_eth export")?;

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
            store,
            instance,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::constants::WASM_PATH;

    use super::*;

    #[tokio::test]
    async fn test_wasm_loader_singleton_loads_module() {
        // First load - should succeed and initialize the singleton
        let loader1 = WasmLoader::new(WASM_PATH).await.unwrap();

        // Second load - should return the same instance
        let loader2 = WasmLoader::new(WASM_PATH).await.unwrap();

        assert!(Arc::ptr_eq(&loader1, &loader2));

        let module_bytes = loader1.module.serialize().unwrap();
        assert!(!module_bytes.is_empty(), "Module bytes should not be empty");
    }

    #[tokio::test]
    async fn test_instantiate_wasm_instance() {
        let loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load wasm module");

        let wasm_instance = loader
            .instantiate()
            .expect("Failed to instantiate wasm instance");

        // Check that memory and all functions are present
        assert!(
            wasm_instance.memory.size(&wasm_instance.store) > 0,
            "Memory size should be > 0"
        );

        // Check alloc function params/results count
        let alloc_ty = wasm_instance.alloc.ty(&wasm_instance.store);
        assert_eq!(alloc_ty.params().len(), 1, "alloc should take 1 parameter");
        assert_eq!(alloc_ty.results().len(), 1, "alloc should return 1 result");

        // Check free function params/results count
        let free_ty = wasm_instance.free.ty(&wasm_instance.store);
        assert_eq!(free_ty.params().len(), 2, "free should take 2 parameters");
        assert_eq!(free_ty.results().len(), 0, "free should return no results");

        // Check public_key function is present and has some params/results
        let public_key_ty = wasm_instance.public_key.ty(&wasm_instance.store);
        assert!(
            public_key_ty.params().len() > 0,
            "public_key should have at least one parameter"
        );

        // Check verify function is present
        let verify_ty = wasm_instance.verify.ty(&wasm_instance.store);
        assert!(
            verify_ty.params().len() > 0,
            "verify should have at least one parameter"
        );

        // Check convert function is present
        let convert_ty = wasm_instance.convert.ty(&wasm_instance.store);
        assert!(
            convert_ty.params().len() > 0,
            "convert should have at least one parameter"
        );

        // Check unconvert function is present
        let unconvert_ty = wasm_instance.unconvert.ty(&wasm_instance.store);
        assert!(
            unconvert_ty.params().len() > 0,
            "unconvert should have at least one parameter"
        );

        // Check recover function is present
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
    }
}
