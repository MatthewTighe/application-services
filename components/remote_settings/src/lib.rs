pub mod error;
pub use error::{RemoteSettingsError, Result};

uniffi_macros::include_scaffolding!("remotesettings");

pub struct RemoteSettings {}

impl RemoteSettings {
    pub fn new() -> Self {
        Self {

        }
    }

    pub fn get(&self) -> Result<Option<String>> {
        todo!();
    }
}
