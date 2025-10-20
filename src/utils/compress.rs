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

/// Dictionary-based compressor for improved compression ratios on similar data
///
/// `DictionaryCompressor` uses Zstandard's dictionary training feature to achieve better
/// compression ratios when compressing multiple files or data chunks that share common
/// patterns. By analyzing sample data during initialization, it builds a dictionary that
/// captures recurring sequences, allowing subsequent compression operations to achieve
/// better compression ratios than standard compression.
///
/// This is particularly useful for dotfiles management where configuration files often
/// share similar structure and syntax (e.g., multiple shell scripts, config files, etc.).
///
/// # Dictionary Training
///
/// The dictionary is trained from sample data provided during construction. The dictionary
/// size is automatically determined based on the total size of samples:
/// - Minimum size: 1KB
/// - Maximum size: 100KB
/// - Default calculation: `total_sample_size` / 4
///
/// # Performance Characteristics
///
/// - **Training overhead**: Dictionary training adds upfront cost during construction
/// - **Compression gain**: 10-30% better compression ratio for similar data patterns
/// - **Memory usage**: Dictionary size (1-100KB) + compression buffers
/// - **Thread safety**: Not thread-safe; create separate instances for concurrent use
///
/// # Examples
///
/// ```no_run
/// use dotman::utils::compress::DictionaryCompressor;
///
/// // Collect sample data from existing dotfiles
/// let samples = vec![
///     b".bashrc content here..." as &[u8],
///     b".zshrc similar content..." as &[u8],
///     b".profile more shell config..." as &[u8],
/// ];
///
/// // Train a dictionary compressor
/// let compressor = DictionaryCompressor::new(&samples, 3)?;
///
/// // Compress new data using the trained dictionary
/// let compressed = compressor.compress(b"new .bashrc content")?;
/// let decompressed = compressor.decompress(&compressed)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub struct DictionaryCompressor {
    /// Trained compression dictionary built from sample data
    ///
    /// The dictionary contains common patterns extracted from sample data during
    /// construction. It is used by both compression and decompression operations
    /// to achieve better compression ratios. An empty dictionary indicates that
    /// dictionary training was skipped (either due to insufficient samples or
    /// training failure), in which case standard compression is used.
    dictionary: Vec<u8>,

    /// Zstandard compression level (1-22)
    ///
    /// Higher levels provide better compression ratios at the cost of slower
    /// compression speed:
    /// - 1-3: Fast compression, lower ratios (good for real-time use)
    /// - 4-9: Balanced compression (recommended for most use cases)
    /// - 10-19: High compression, slower (good for archival)
    /// - 20-22: Ultra compression, very slow (maximum compression)
    ///
    /// This level is used for both dictionary-based and fallback compression.
    level: i32,
}

impl DictionaryCompressor {
    /// Create a new dictionary compressor from sample data
    ///
    /// Trains a compression dictionary by analyzing the provided sample data. The dictionary
    /// is optimized to capture common patterns across all samples, improving compression ratios
    /// for data with similar characteristics.
    ///
    /// # Arguments
    ///
    /// * `sample_data` - Slice of byte slices representing sample data to train on. Should
    ///   contain representative examples of the data you plan to compress. More diverse samples
    ///   generally produce better dictionaries. An empty slice will result in standard
    ///   compression without dictionary support.
    /// * `level` - Zstandard compression level (1-22). Higher values produce better compression
    ///   but are slower. Recommended: 3 for fast, 9 for balanced, 19 for maximum compression.
    ///
    /// # Returns
    ///
    /// A new `DictionaryCompressor` instance with a trained dictionary (or empty dictionary
    /// if training was not possible).
    ///
    /// # Errors
    ///
    /// Currently this function cannot fail, but returns `Result` for future compatibility
    /// with potential training errors or validation failures.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use dotman::utils::compress::DictionaryCompressor;
    ///
    /// // Train on shell configuration files
    /// let samples = vec![
    ///     include_bytes!("../fixtures/.bashrc") as &[u8],
    ///     include_bytes!("../fixtures/.zshrc") as &[u8],
    /// ];
    /// let compressor = DictionaryCompressor::new(&samples, 9)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
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
    /// Compresses the input data using the trained dictionary for improved compression ratios.
    /// If no dictionary was trained (empty dictionary), falls back to standard Zstandard
    /// compression at the configured level.
    ///
    /// # Arguments
    ///
    /// * `data` - The raw data to compress. Can be of any size, though dictionary-based
    ///   compression works best with data similar to the training samples.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the compressed data. The output is self-contained and includes
    /// dictionary references internally (if applicable).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Zstandard encoder initialization fails
    /// - Writing to the compression stream fails
    /// - Finalizing the compression stream fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use dotman::utils::compress::DictionaryCompressor;
    ///
    /// let samples = vec![b"sample1" as &[u8], b"sample2" as &[u8]];
    /// let compressor = DictionaryCompressor::new(&samples, 3)?;
    ///
    /// let data = b"data to compress";
    /// let compressed = compressor.compress(data)?;
    /// assert!(compressed.len() <= data.len());
    /// # Ok::<(), anyhow::Error>(())
    /// ```
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
    /// Decompresses data that was previously compressed with this dictionary compressor's
    /// `compress` method. The same dictionary used for compression must be used for
    /// decompression, otherwise the operation will fail or produce corrupted output.
    ///
    /// # Arguments
    ///
    /// * `data` - The compressed data to decompress. Must have been compressed using the
    ///   same dictionary (or no dictionary if this compressor has an empty dictionary).
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the decompressed data, which should match the original
    /// uncompressed data byte-for-byte.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The data was not compressed with a compatible dictionary
    /// - The data is corrupted or invalid
    /// - Zstandard decoder initialization fails
    /// - Reading from the decompression stream fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use dotman::utils::compress::DictionaryCompressor;
    ///
    /// let samples = vec![b"sample1" as &[u8], b"sample2" as &[u8]];
    /// let compressor = DictionaryCompressor::new(&samples, 3)?;
    ///
    /// let original = b"data to compress";
    /// let compressed = compressor.compress(original)?;
    /// let decompressed = compressor.decompress(&compressed)?;
    /// assert_eq!(original, &decompressed[..]);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
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
