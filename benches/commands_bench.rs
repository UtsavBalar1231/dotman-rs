use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dotman::DotmanContext;
use dotman::commands;
use dotman::config::Config;
use std::fs;
use std::hint::black_box;
use tempfile::tempdir;

fn setup_test_context() -> (tempfile::TempDir, DotmanContext) {
    let dir = tempdir().unwrap();
    let repo_path = dir.path().join(".dotman");
    let config_path = dir.path().join("config.toml");

    // Create repo structure
    fs::create_dir_all(&repo_path).unwrap();
    fs::create_dir_all(repo_path.join("commits")).unwrap();
    fs::create_dir_all(repo_path.join("objects")).unwrap();

    let mut config = Config::default();
    config.core.repo_path.clone_from(&repo_path);
    config.save(&config_path).unwrap();

    let context = DotmanContext {
        repo_path,
        config_path,
        config,
        no_pager: true,
    };

    (dir, context)
}

fn create_test_files_in_dir(dir: &std::path::Path, count: usize) -> Vec<String> {
    let mut paths = Vec::new();

    for i in 0..count {
        let path = dir.join(format!("test_file_{i}.txt"));
        let content = format!("Content of file {i}\n").repeat(10);
        fs::write(&path, content).unwrap();
        paths.push(path.to_string_lossy().to_string());
    }

    paths
}

fn benchmark_add_command(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_command");

    for file_count in &[1, 10, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(file_count),
            file_count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let (dir, ctx) = setup_test_context();
                        let files = create_test_files_in_dir(dir.path(), count);
                        (dir, ctx, files)
                    },
                    |(dir, ctx, files)| {
                        commands::add::execute(&ctx, &files, false).unwrap();
                        drop(dir); // Cleanup
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn benchmark_status_command(c: &mut Criterion) {
    let mut group = c.benchmark_group("status_command");

    // Setup with various file counts
    for file_count in &[10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(file_count),
            file_count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let (dir, ctx) = setup_test_context();
                        let files = create_test_files_in_dir(dir.path(), count);
                        // Add files to index
                        commands::add::execute(&ctx, &files, false).unwrap();
                        (dir, ctx)
                    },
                    |(dir, ctx)| {
                        commands::status::execute(&ctx, false, false).ok();
                        drop(dir);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn benchmark_commit_command(c: &mut Criterion) {
    let mut group = c.benchmark_group("commit_command");

    for file_count in &[10, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(file_count),
            file_count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let (dir, ctx) = setup_test_context();
                        let files = create_test_files_in_dir(dir.path(), count);
                        commands::add::execute(&ctx, &files, false).unwrap();
                        (dir, ctx)
                    },
                    |(dir, ctx)| {
                        commands::commit::execute(&ctx, "Test commit", false).unwrap();
                        drop(dir);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn benchmark_utils(c: &mut Criterion) {
    use dotman::utils::{format_size, should_ignore};

    let mut group = c.benchmark_group("utils");

    // Benchmark format_size
    group.bench_function("format_size", |b| {
        b.iter(|| {
            let _ = format_size(black_box(1024));
            let _ = format_size(black_box(1_048_576));
            let _ = format_size(black_box(1_073_741_824));
        });
    });

    // Benchmark should_ignore
    let patterns = vec![
        "*.swp".to_string(),
        ".git".to_string(),
        "node_modules".to_string(),
        "*.tmp".to_string(),
        "__pycache__".to_string(),
    ];

    let test_paths = vec![
        std::path::Path::new("file.swp"),
        std::path::Path::new("normal.txt"),
        std::path::Path::new(".git/config"),
        std::path::Path::new("src/main.rs"),
    ];

    group.bench_function("should_ignore", |b| {
        b.iter(|| {
            for path in &test_paths {
                let _ = should_ignore(black_box(path), black_box(&patterns));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_add_command,
    benchmark_status_command,
    benchmark_commit_command,
    benchmark_utils
);
criterion_main!(benches);
