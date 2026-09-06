//! Keep the SpacetimeDB access token so a relaunch is the same wanderer.

use bevy_stdb::prelude::ReadStdbConnectedMessage;

#[cfg(target_arch = "wasm32")]
const TOKEN_KEY: &str = "unbound.identity.token";

pub fn load_token() -> Option<String> {
    let token = load_token_raw()?.trim().to_string();
    if token.is_empty() { None } else { Some(token) }
}

pub fn persist_on_connect(mut connected: ReadStdbConnectedMessage) {
    for msg in connected.read() {
        save_token(&msg.access_token);
    }
}

fn save_token(token: &str) {
    if token.is_empty() {
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = token_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, token);
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = wasm_storage() {
            let _ = storage.set_item(TOKEN_KEY, token);
        }
    }
}

fn load_token_raw() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(token_path()).ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        wasm_storage()?.get_item(TOKEN_KEY).ok().flatten()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn token_path() -> std::path::PathBuf {
    let mut dir = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| std::path::PathBuf::from(home).join(".local").join("share"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    dir.push("unbound");
    dir.join("identity.token")
}

#[cfg(target_arch = "wasm32")]
fn wasm_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn token_path_lives_under_unbound() {
        let path = token_path();
        assert!(path.ends_with("identity.token"));
        assert!(path.components().any(|c| c.as_os_str() == "unbound"));
    }
}
