use wasm_bindgen::prelude::*;
use idb::Factory;

#[wasm_bindgen]
pub async fn test_indexeddb() -> String {
    // Just test if we can create a factory - the actual DB ops will require browser
    match Factory::new() {
        Ok(_factory) => "IndexedDB Factory created successfully".to_string(),
        Err(e) => format!("Error creating factory: {:?}", e),
    }
}
