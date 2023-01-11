#[derive(Debug, thiserror::Error)]
pub enum RemoteSettingsError {
    #[error("Network failure")]
    NetworkFailure
}

pub type Result<T, E = RemoteSettingsError> = std::result::Result<T, E>;