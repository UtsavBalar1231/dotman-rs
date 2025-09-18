use anyhow::Result;

/// Get the bincode configuration
fn get_config() -> impl bincode::config::Config {
    // Use legacy configuration for better compatibility with serde
    // Limit allocation to prevent memory exhaustion on corrupt data
    bincode::config::legacy().with_limit::<{ 100 * 1024 * 1024 }>() // 100MB limit
}

/// Serialize data using bincode v2.0 with serde
///
/// # Errors
///
/// Returns an error if:
/// - Serialization fails
pub fn serialize<T: serde::Serialize>(data: &T) -> Result<Vec<u8>> {
    bincode::serde::encode_to_vec(data, get_config()).map_err(Into::into)
}

/// Deserialize data using bincode v2.0 with serde
///
/// # Errors
///
/// Returns an error if:
/// - Deserialization fails
/// - Data is malformed or incompatible
pub fn deserialize<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let (result, _bytes_read) = bincode::serde::decode_from_slice(bytes, get_config())?;
    Ok(result)
}
