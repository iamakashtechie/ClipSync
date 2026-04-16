use std::hash::{Hash, Hasher};

use crate::domain::state::AppState;

pub fn compute_text_hash(sender: &str, text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    sender.hash(&mut hasher);
    text.hash(&mut hasher);
    hasher.finish()
}

pub fn compute_image_hash(sender: &str, mime_type: &str, image_base64: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    sender.hash(&mut hasher);
    mime_type.hash(&mut hasher);
    image_base64.hash(&mut hasher);
    hasher.finish()
}

pub fn remember_hash(state: &mut AppState, hash: u64) {
    const MAX_RECENT_HASHES: usize = 64;
    if state.recent_hashes.contains(&hash) {
        return;
    }
    state.recent_hashes.push_back(hash);
    if state.recent_hashes.len() > MAX_RECENT_HASHES {
        state.recent_hashes.pop_front();
    }
}
