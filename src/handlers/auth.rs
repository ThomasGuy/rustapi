use std::sync::{Arc, RwLock};

use axum::extract::State;

use crate::utils::app_state::TokenKeys;

// HANDLER: Reading keys
async fn login(State(keys): State<Arc<RwLock<TokenKeys>>>) {
    let current_keys = keys.read().unwrap(); // Shared read access
    println!("Current key: {}", current_keys.token_key);
}

// REFRESH TASK: Writing keys
async fn refresh_loop(keys: Arc<RwLock<TokenKeys>>) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        let mut write_guard = keys.write().unwrap(); // Exclusive write access
        write_guard.token_key = "new_key_data".to_string();
    }
}
