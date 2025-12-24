use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn test_sled() -> String {
    // Try to create an in-memory sled database
    match sled::Config::new().temporary(true).open() {
        Ok(db) => {
            // Try some basic operations
            match db.insert("key", "value") {
                Ok(_) => match db.get("key") {
                    Ok(Some(value)) => {
                        format!("Sled works! Got value: {:?}", String::from_utf8_lossy(&value))
                    }
                    Ok(None) => "Sled works but key not found".to_string(),
                    Err(e) => format!("Sled get error: {:?}", e),
                },
                Err(e) => format!("Sled insert error: {:?}", e),
            }
        }
        Err(e) => format!("Failed to open sled: {:?}", e),
    }
}
