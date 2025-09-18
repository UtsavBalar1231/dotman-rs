use anyhow::Result;
use rayon::prelude::*;
use std::io::{Read, Write};
use zstd::stream::{Decoder, Encoder};

/// Compress bytes using zstd compression
///
/// # Errors
///
/// Returns an error if compression fails
pub fn compress_bytes(data: &[u8], level: i32) -> Result<Vec<u8>> {
    zstd::encode_all(data, level).map_err(Into::into)
}

/// Decompress bytes compressed with zstd
///
/// # Errors
///
/// Returns an error if decompression fails
pub fn decompress_bytes(data: &[u8]) -> Result<Vec<u8>> {
    zstd::decode_all(data).map_err(Into::into)
}

/// Compress a file using zstd compression
///
/// # Errors
///
/// Returns an error if:
/// - Input file cannot be opened
/// - Output file cannot be created
/// - Compression fails
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

/// Decompress a zstd compressed file
///
/// # Errors
///
/// Returns an error if:
/// - Input file cannot be opened
/// - Output file cannot be created
/// - Decompression fails
pub fn decompress_file(input_path: &std::path::Path, output_path: &std::path::Path) -> Result<u64> {
    let input = std::fs::File::open(input_path)?;
    let output = std::fs::File::create(output_path)?;

    let mut decoder = Decoder::new(input)?;
    let bytes_written = std::io::copy(&mut decoder, &mut std::io::BufWriter::new(output))?;

    Ok(bytes_written)
}

/// Compress multiple files in parallel
///
/// # Errors
///
/// Returns a vector of results, each of which may contain an error if compression fails
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
    /// Create a new dictionary compressor from sample data
    ///
    /// # Errors
    ///
    /// Currently this function cannot fail, but returns Result for future compatibility
    pub fn new(sample_data: &[&[u8]], level: i32) -> Result<Self> {
        let dictionary = if sample_data.is_empty() {
            Vec::new()
        } else {
            // Calculate appropriate dictionary size based on samples
            let total_size: usize = sample_data.iter().map(|s| s.len()).sum();
            let dict_size = (total_size / 4).clamp(1024, 102_400); // Between 1KB and 100KB

            // Try to create dictionary, fall back to empty if samples are too small
            zstd::dict::from_samples(sample_data, dict_size).unwrap_or_default()
        };

        Ok(Self { dictionary, level })
    }

    /// Compress data using the trained dictionary
    ///
    /// # Errors
    ///
    /// Returns an error if compression fails
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

    /// Decompress data using the trained dictionary
    ///
    /// # Errors
    ///
    /// Returns an error if decompression fails
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

/// Estimate the compression ratio for given data and level
///
/// # Errors
///
/// Returns an error if compression fails
#[allow(clippy::cast_precision_loss)]
pub fn estimate_compression_ratio(data: &[u8], level: i32) -> Result<f64> {
    let compressed = compress_bytes(data, level)?;
    Ok(compressed.len() as f64 / data.len() as f64)
}

/// Find the optimal compression level within a time budget
///
/// # Errors
///
/// Returns an error if compression testing fails
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

        #[allow(clippy::cast_precision_loss)]
        let ratio = compressed.len() as f64 / data.len() as f64;
        if ratio < best_ratio {
            best_ratio = ratio;
            best_level = level;
        }
    }

    Ok(best_level)
}
