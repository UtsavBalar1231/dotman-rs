use anyhow::Result;

/// Get the standard bincode configuration for optimal performance
fn get_config() -> impl bincode::config::Config {
    bincode::config::standard()
}

/// Serialize data using bincode v2.0 with serde
pub fn serialize<T: serde::Serialize>(data: &T) -> Result<Vec<u8>> {
    bincode::serde::encode_to_vec(data, get_config()).map_err(Into::into)
}

/// Deserialize data using bincode v2.0 with serde
pub fn deserialize<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let (result, _) = bincode::serde::decode_from_slice(bytes, get_config())?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        id: u32,
        name: String,
        data: Vec<u8>,
    }

    #[test]
    fn test_serialize_deserialize() -> Result<()> {
        let original = TestStruct {
            id: 42,
            name: "test".to_string(),
            data: vec![1, 2, 3, 4, 5],
        };

        let serialized = serialize(&original)?;
        let deserialized: TestStruct = deserialize(&serialized)?;

        assert_eq!(original, deserialized);
        Ok(())
    }

    #[test]
    fn test_serialize_empty_vec() -> Result<()> {
        let empty_vec: Vec<u32> = vec![];
        let serialized = serialize(&empty_vec)?;
        let deserialized: Vec<u32> = deserialize(&serialized)?;

        assert_eq!(empty_vec, deserialized);
        Ok(())
    }
}
