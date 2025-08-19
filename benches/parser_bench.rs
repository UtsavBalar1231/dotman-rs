use criterion::{Criterion, criterion_group, criterion_main};
use dotman::config::parser::parse_config_file;
use std::fs;
use std::hint::black_box;
use tempfile::tempdir;

fn create_test_config(size: usize) -> String {
    let mut config = String::from("[core]\n");
    config.push_str("repo_path = \"~/.dotman\"\n");
    config.push_str("default_branch = \"main\"\n");
    config.push_str("compression = \"zstd\"\n");
    config.push_str("compression_level = 3\n\n");

    config.push_str("[performance]\n");
    config.push_str("parallel_threads = 8\n");
    config.push_str("mmap_threshold = 1048576\n");
    config.push_str("cache_size = 100\n\n");

    config.push_str("[tracking]\n");
    config.push_str("ignore_patterns = [\n");

    // Add many ignore patterns to increase config size
    for i in 0..size {
        config.push_str(&format!("  \"pattern_{}\",\n", i));
    }

    config.push_str("]\n");
    config.push_str("follow_symlinks = false\n");
    config.push_str("preserve_permissions = true\n");

    config
}

fn benchmark_config_parsing(c: &mut Criterion) {
    let dir = tempdir().unwrap();

    // Small config
    let small_config = create_test_config(10);
    let small_path = dir.path().join("small.toml");
    fs::write(&small_path, &small_config).unwrap();

    // Medium config
    let medium_config = create_test_config(100);
    let medium_path = dir.path().join("medium.toml");
    fs::write(&medium_path, &medium_config).unwrap();

    // Large config
    let large_config = create_test_config(1000);
    let large_path = dir.path().join("large.toml");
    fs::write(&large_path, &large_config).unwrap();

    let mut group = c.benchmark_group("config_parsing");

    group.bench_function("small_config", |b| {
        b.iter(|| parse_config_file(black_box(&small_path)))
    });

    group.bench_function("medium_config", |b| {
        b.iter(|| parse_config_file(black_box(&medium_path)))
    });

    group.bench_function("large_config", |b| {
        b.iter(|| parse_config_file(black_box(&large_path)))
    });

    group.finish();
}

fn benchmark_simd_validation(c: &mut Criterion) {
    let valid_utf8 = vec![b'a'; 10000];
    let invalid_utf8 = {
        let mut v = vec![b'a'; 10000];
        v[5000] = 0xFF;
        v
    };

    let mut group = c.benchmark_group("simd_utf8_validation");

    group.bench_function("valid_10kb", |b| {
        b.iter(|| simdutf8::basic::from_utf8(black_box(&valid_utf8)))
    });

    group.bench_function("invalid_10kb", |b| {
        b.iter(|| simdutf8::basic::from_utf8(black_box(&invalid_utf8)).is_err())
    });

    group.finish();
}

criterion_group!(benches, benchmark_config_parsing, benchmark_simd_validation);
criterion_main!(benches);
