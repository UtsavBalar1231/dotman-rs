use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dotman::storage::FileEntry;
use dotman::storage::file_ops::{hash_file, hash_files_parallel};
use dotman::storage::index::Index;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use tempfile::tempdir;

fn create_test_files(dir: &std::path::Path, count: usize) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for i in 0..count {
        let path = dir.join(format!("file_{i}.txt"));
        let content = format!("This is test file number {i} with some content to hash");
        fs::write(&path, content).unwrap();
        paths.push(path);
    }

    paths
}

fn benchmark_hashing(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let small_file = dir.path().join("small.txt");
    let medium_file = dir.path().join("medium.txt");
    let large_file = dir.path().join("large.txt");

    fs::write(&small_file, vec![b'a'; 1024]).unwrap(); // 1KB
    fs::write(&medium_file, vec![b'b'; 1024 * 100]).unwrap(); // 100KB
    fs::write(&large_file, vec![b'c'; 1024 * 1024 * 10]).unwrap(); // 10MB

    let mut group = c.benchmark_group("file_hashing");

    group.bench_function("hash_1kb", |b| b.iter(|| hash_file(black_box(&small_file))));

    group.bench_function("hash_100kb", |b| {
        b.iter(|| hash_file(black_box(&medium_file)));
    });

    group.bench_function("hash_10mb", |b| {
        b.iter(|| hash_file(black_box(&large_file)));
    });

    group.finish();
}

fn benchmark_parallel_hashing(c: &mut Criterion) {
    let dir = tempdir().unwrap();

    let mut group = c.benchmark_group("parallel_hashing");

    for count in &[10, 50, 100] {
        let files = create_test_files(dir.path(), *count);

        group.bench_with_input(BenchmarkId::from_parameter(count), &files, |b, files| {
            b.iter(|| hash_files_parallel(black_box(files)));
        });
    }

    group.finish();
}

fn benchmark_index_operations(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let index_path = dir.path().join("index.bin");

    // Create test entries
    let entries: Vec<FileEntry> = (0..1000)
        .map(|i| FileEntry {
            path: PathBuf::from(format!("/home/user/file_{i}.txt")),
            hash: format!("{i:032x}"),
            size: u64::try_from(i).unwrap_or(0) * 100,
            modified: 1_234_567_890 + i64::from(i),
            mode: 0o644,
        })
        .collect();

    let mut group = c.benchmark_group("index_operations");

    // Benchmark index creation and population
    group.bench_function("index_add_1000", |b| {
        b.iter(|| {
            let mut index = Index::new();
            for entry in &entries {
                index.add_entry(entry.clone());
            }
        });
    });

    // Benchmark parallel index operations
    group.bench_function("index_add_parallel_1000", |b| {
        b.iter(|| {
            let mut index = Index::new();
            index.add_entries_parallel(entries.clone());
        });
    });

    // Benchmark index serialization
    let mut index = Index::new();
    for entry in &entries {
        index.add_entry(entry.clone());
    }

    group.bench_function("index_save", |b| {
        b.iter(|| index.save(black_box(&index_path)));
    });

    // Save index for loading benchmark
    index.save(&index_path).unwrap();

    group.bench_function("index_load", |b| {
        b.iter(|| Index::load(black_box(&index_path)));
    });

    group.finish();
}

fn benchmark_compression(c: &mut Criterion) {
    use dotman::utils::compress::{compress_bytes, decompress_bytes};

    let small_data = vec![b'a'; 1024]; // 1KB
    let medium_data = vec![b'b'; 1024 * 100]; // 100KB
    let large_data = vec![b'c'; 1024 * 1024]; // 1MB

    let mut group = c.benchmark_group("compression");

    for level in &[1, 3, 9] {
        group.bench_function(format!("compress_1kb_level_{level}"), |b| {
            b.iter(|| compress_bytes(black_box(&small_data), *level));
        });

        group.bench_function(format!("compress_100kb_level_{level}"), |b| {
            b.iter(|| compress_bytes(black_box(&medium_data), *level));
        });

        group.bench_function(format!("compress_1mb_level_{level}"), |b| {
            b.iter(|| compress_bytes(black_box(&large_data), *level));
        });
    }

    // Benchmark decompression
    let compressed_small = compress_bytes(&small_data, 3).unwrap();
    let compressed_medium = compress_bytes(&medium_data, 3).unwrap();
    let compressed_large = compress_bytes(&large_data, 3).unwrap();

    group.bench_function("decompress_1kb", |b| {
        b.iter(|| decompress_bytes(black_box(&compressed_small)));
    });

    group.bench_function("decompress_100kb", |b| {
        b.iter(|| decompress_bytes(black_box(&compressed_medium)));
    });

    group.bench_function("decompress_1mb", |b| {
        b.iter(|| decompress_bytes(black_box(&compressed_large)));
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_hashing,
    benchmark_parallel_hashing,
    benchmark_index_operations,
    benchmark_compression
);
criterion_main!(benches);
