use anyhow::Result;
use rayon::prelude::*;
use std::io::{Read, Write};
use zstd::stream::{Decoder, Encoder};

pub fn compress_bytes(data: &[u8], level: i32) -> Result<Vec<u8>> {
    zstd::encode_all(data, level).map_err(Into::into)
}

pub fn decompress_bytes(data: &[u8]) -> Result<Vec<u8>> {
    zstd::decode_all(data).map_err(Into::into)
}

pub fn compress_file(
    input_path: &std::path::Path,
    output_path: &std::path::Path,
    level: i32,
) -> Result<u64> {
    let input = std::fs::File::open(input_path)?;
    let output = std::fs::File::create(output_path)?;

    let mut encoder = Encoder::new(output, level)?;
    let bytes_written = std::io::copy(&mut std::io::BufReader::new(input), &mut encoder)?;
    encoder.finish()?;

    Ok(bytes_written)
}

pub fn decompress_file(input_path: &std::path::Path, output_path: &std::path::Path) -> Result<u64> {
    let input = std::fs::File::open(input_path)?;
    let output = std::fs::File::create(output_path)?;

    let mut decoder = Decoder::new(input)?;
    let bytes_written = std::io::copy(&mut decoder, &mut std::io::BufWriter::new(output))?;

    Ok(bytes_written)
}

pub fn compress_files_parallel(
    files: &[(std::path::PathBuf, std::path::PathBuf)],
    level: i32,
) -> Result<Vec<Result<u64>>> {
    Ok(files
        .par_iter()
        .map(|(input, output)| compress_file(input, output, level))
        .collect())
}

// Advanced compression with dictionary training for better ratio
pub struct DictionaryCompressor {
    dictionary: Vec<u8>,
    level: i32,
}

impl DictionaryCompressor {
    pub fn new(sample_data: &[&[u8]], level: i32) -> Result<Self> {
        let dictionary = if !sample_data.is_empty() {
            // Calculate appropriate dictionary size based on samples
            let total_size: usize = sample_data.iter().map(|s| s.len()).sum();
            let dict_size = (total_size / 4).clamp(1024, 102400); // Between 1KB and 100KB

            // Try to create dictionary, fall back to empty if samples are too small
            zstd::dict::from_samples(sample_data, dict_size).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self { dictionary, level })
    }

    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if self.dictionary.is_empty() {
            compress_bytes(data, self.level)
        } else {
            let mut encoder =
                zstd::Encoder::with_dictionary(Vec::new(), self.level, &self.dictionary)?;
            encoder.write_all(data)?;
            encoder.finish().map_err(Into::into)
        }
    }

    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if self.dictionary.is_empty() {
            decompress_bytes(data)
        } else {
            let mut decoder = zstd::Decoder::with_dictionary(data, &self.dictionary)?;
            let mut result = Vec::new();
            decoder.read_to_end(&mut result)?;
            Ok(result)
        }
    }
}

pub fn estimate_compression_ratio(data: &[u8], level: i32) -> Result<f64> {
    let compressed = compress_bytes(data, level)?;
    Ok(compressed.len() as f64 / data.len() as f64)
}

pub fn find_optimal_level(data: &[u8], max_time_ms: u128) -> Result<i32> {
    use std::time::Instant;

    let mut best_level = 1;
    let mut best_ratio = 1.0;

    for level in 1..=22 {
        let start = Instant::now();
        let compressed = compress_bytes(data, level)?;
        let elapsed = start.elapsed().as_millis();

        if elapsed > max_time_ms {
            break;
        }

        let ratio = compressed.len() as f64 / data.len() as f64;
        if ratio < best_ratio {
            best_ratio = ratio;
            best_level = level;
        }
    }

    Ok(best_level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_compress_decompress_bytes() -> Result<()> {
        // Use a longer, more repetitive string that will compress well
        let original = b"Hello, World! This is a test string for compression. This is a test string for compression. This is a test string for compression.";
        let compressed = compress_bytes(original, 3)?;
        let decompressed = decompress_bytes(&compressed)?;

        assert_eq!(original.to_vec(), decompressed);
        // Compression might not reduce size for small strings, so just verify it works
        assert!(!compressed.is_empty());

        Ok(())
    }

    #[test]
    fn test_compress_decompress_file() -> Result<()> {
        let dir = tempdir()?;
        let input_path = dir.path().join("input.txt");
        let compressed_path = dir.path().join("compressed.zst");
        let output_path = dir.path().join("output.txt");

        let content = "This is test content for file compression.\n".repeat(100);
        std::fs::write(&input_path, &content)?;

        compress_file(&input_path, &compressed_path, 3)?;
        decompress_file(&compressed_path, &output_path)?;

        let decompressed_content = std::fs::read_to_string(&output_path)?;
        assert_eq!(content, decompressed_content);

        let compressed_size = std::fs::metadata(&compressed_path)?.len();
        let original_size = content.len() as u64;
        assert!(compressed_size < original_size);

        Ok(())
    }

    #[test]
    fn test_dictionary_compressor() -> Result<()> {
        // Test with empty dictionary (should fall back to regular compression)
        let compressor = DictionaryCompressor::new(&[], 3)?;

        let data = b"Test data for compression with dictionary compressor";
        let compressed = compressor.compress(data)?;
        let decompressed = compressor.decompress(&compressed)?;

        assert_eq!(data.to_vec(), decompressed);

        // Test with samples for dictionary
        let samples = vec![
            b"test sample data test sample".as_ref(),
            b"sample data for testing".as_ref(),
        ];

        let compressor_with_dict = DictionaryCompressor::new(&samples, 3)?;

        let test_data = b"test sample for compression";
        let compressed_dict = compressor_with_dict.compress(test_data)?;
        let decompressed_dict = compressor_with_dict.decompress(&compressed_dict)?;

        assert_eq!(test_data.to_vec(), decompressed_dict);

        Ok(())
    }

    #[test]
    fn test_estimate_compression_ratio() -> Result<()> {
        let data = b"Repeated data repeated data repeated data".as_ref();
        let ratio = estimate_compression_ratio(data, 3)?;

        assert!(ratio < 1.0); // Should compress well due to repetition
        assert!(ratio > 0.0);

        Ok(())
    }

    #[test]
    fn test_find_optimal_level() -> Result<()> {
        let data =
            b"Test data for finding optimal compression level with some repetition repetition"
                .as_ref();
        let level = find_optimal_level(data, 100)?; // 100ms max time

        assert!(level >= 1);
        assert!(level <= 22);

        Ok(())
    }
}
