pub mod error;
use config::RemoteSettingsConfig;
pub use error::{RemoteSettingsError, Result};
mod config;

uniffi_macros::include_scaffolding!("remotesettings");

pub struct RemoteSettings {
    pub config: RemoteSettingsConfig,
}

impl RemoteSettings {

    pub fn new(_config: Option<RemoteSettingsConfig>) -> Self {
        todo!();
        // let config: RemoteSettingsConfig = config.unwrap_or_else(|| RemoteSettingsConfig { server_url: String::from(""), collection_name: String::from(""), bucket_name: Some(String::from("")) });
        // Self {
            // config,
        // }
    }

    pub fn get(&self) -> Result<Option<String>> {
        todo!();
    }
}
