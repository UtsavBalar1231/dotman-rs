use anyhow::Result;
use dotman::DotmanContext;
use dotman::commands;
use dotman::config::Config;
use std::fs;
use tempfile::tempdir;

// Helper to setup a test context
fn setup_test_env() -> Result<(tempfile::TempDir, DotmanContext)> {
    let dir = tempdir()?;

    // Set HOME to temp dir for test isolation
    unsafe {
        std::env::set_var("HOME", dir.path());
    }

    // Initialize repository
    commands::init::execute(false)?;

    let repo_path = dir.path().join(".dotman");
    let config_path = dir.path().join(".config/dotman/config");
    let config = Config::load(&config_path)?;

    let ctx = DotmanContext {
        repo_path,
        config_path,
        config,
    };

    Ok((dir, ctx))
}

#[test]
fn test_full_workflow_init_add_commit_checkout() -> Result<()> {
    let (dir, ctx) = setup_test_env()?;

    // Create test files
    let file1 = dir.path().join("config.toml");
    let file2 = dir.path().join("settings.json");
    let file3 = dir.path().join(".bashrc");

    fs::write(&file1, "config = true\nvalue = 42")?;
    fs::write(&file2, r#"{"setting": "value", "number": 123}"#)?;
    fs::write(
        &file3,
        "export PATH=$PATH:/usr/local/bin\nalias ll='ls -la'",
    )?;

    // Add files
    let paths = vec![
        file1.to_string_lossy().to_string(),
        file2.to_string_lossy().to_string(),
        file3.to_string_lossy().to_string(),
    ];
    commands::add::execute(&ctx, &paths, false)?;

    // Check status
    commands::status::execute(&ctx, false)?;

    // Commit
    commands::commit::execute(&ctx, "Initial configuration files", false)?;

    // Verify HEAD exists
    let head_path = ctx.repo_path.join("HEAD");
    assert!(head_path.exists());
    let first_commit = fs::read_to_string(&head_path)?;

    // Modify files
    fs::write(&file1, "config = false\nvalue = 100\nnew_field = true")?;
    fs::write(
        &file2,
        r#"{"setting": "changed", "number": 456, "new": true}"#,
    )?;

    // Add and commit changes
    commands::add::execute(&ctx, &paths[0..2].to_vec(), false)?;
    commands::commit::execute(&ctx, "Update configuration", false)?;

    let second_commit = fs::read_to_string(&head_path)?;
    assert_ne!(first_commit, second_commit);

    // Checkout first commit
    commands::checkout::execute(&ctx, &first_commit.trim(), true)?;

    // Verify files were restored
    let restored_content1 = fs::read_to_string(&file1)?;
    assert!(restored_content1.contains("config = true"));
    assert!(restored_content1.contains("value = 42"));
    assert!(!restored_content1.contains("new_field"));

    let restored_content2 = fs::read_to_string(&file2)?;
    assert!(restored_content2.contains(r#""setting": "value""#));
    assert!(restored_content2.contains(r#""number": 123"#));
    assert!(!restored_content2.contains(r#""new": true"#));

    // Show log
    commands::log::execute(&ctx, 10, false)?;

    Ok(())
}

#[test]
fn test_workflow_with_directories() -> Result<()> {
    let (dir, ctx) = setup_test_env()?;

    // Create directory structure
    let config_dir = dir.path().join(".config");
    let nvim_dir = config_dir.join("nvim");
    let lua_dir = nvim_dir.join("lua");

    fs::create_dir_all(&lua_dir)?;

    // Create nested files
    fs::write(nvim_dir.join("init.vim"), "set number\nset expandtab")?;
    fs::write(lua_dir.join("config.lua"), "vim.opt.number = true")?;
    fs::write(config_dir.join("gitconfig"), "[user]\nname = Test")?;

    // Add entire config directory
    let paths = vec![config_dir.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Commit
    commands::commit::execute(&ctx, "Add config directory", false)?;

    // Show what was committed
    let head = fs::read_to_string(ctx.repo_path.join("HEAD"))?;
    commands::show::execute(&ctx, &head.trim())?;

    Ok(())
}

#[test]
fn test_reset_workflow() -> Result<()> {
    let (dir, ctx) = setup_test_env()?;

    // Create and add file
    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "version 1")?;

    let paths = vec![test_file.to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;
    commands::commit::execute(&ctx, "First commit", false)?;

    let first_commit = fs::read_to_string(ctx.repo_path.join("HEAD"))?
        .trim()
        .to_string();

    // Make second commit
    fs::write(&test_file, "version 2")?;
    commands::add::execute(&ctx, &paths, false)?;
    commands::commit::execute(&ctx, "Second commit", false)?;

    let second_commit = fs::read_to_string(ctx.repo_path.join("HEAD"))?
        .trim()
        .to_string();

    // Make third commit
    fs::write(&test_file, "version 3")?;
    commands::add::execute(&ctx, &paths, false)?;
    commands::commit::execute(&ctx, "Third commit", false)?;

    // Soft reset to second commit (keeps working directory)
    commands::reset::execute(&ctx, &second_commit, false, true)?;

    // File should still have version 3
    let content = fs::read_to_string(&test_file)?;
    assert_eq!(content, "version 3");

    // HEAD should point to second commit
    let head = fs::read_to_string(ctx.repo_path.join("HEAD"))?
        .trim()
        .to_string();
    assert_eq!(head, second_commit);

    // Hard reset to first commit
    commands::reset::execute(&ctx, &first_commit, true, false)?;

    // File should now have version 1
    let content = fs::read_to_string(&test_file)?;
    assert_eq!(content, "version 1");

    Ok(())
}

#[test]
fn test_diff_workflow() -> Result<()> {
    let (dir, ctx) = setup_test_env()?;

    // Create files
    let file1 = dir.path().join("file1.txt");
    let file2 = dir.path().join("file2.txt");

    fs::write(&file1, "line 1\nline 2\nline 3")?;
    fs::write(&file2, "content A")?;

    // Add and commit
    let paths = vec![
        file1.to_string_lossy().to_string(),
        file2.to_string_lossy().to_string(),
    ];
    commands::add::execute(&ctx, &paths, false)?;
    commands::commit::execute(&ctx, "Initial files", false)?;

    let first_commit = fs::read_to_string(ctx.repo_path.join("HEAD"))?
        .trim()
        .to_string();

    // Modify files
    fs::write(&file1, "line 1\nline 2 modified\nline 3\nline 4")?;
    fs::remove_file(&file2)?;

    // Create new file
    let file3 = dir.path().join("file3.txt");
    fs::write(&file3, "new file")?;

    // Diff working vs index
    commands::diff::execute(&ctx, None, None)?;

    // Add changes and commit
    commands::add::execute(&ctx, &[file1.to_string_lossy().to_string()], false)?;
    commands::add::execute(&ctx, &[file3.to_string_lossy().to_string()], false)?;
    commands::commit::execute(&ctx, "Modifications", false)?;

    let second_commit = fs::read_to_string(ctx.repo_path.join("HEAD"))?
        .trim()
        .to_string();

    // Diff between commits
    commands::diff::execute(&ctx, Some(&first_commit), Some(&second_commit))?;

    Ok(())
}

#[test]
fn test_rm_workflow() -> Result<()> {
    let (dir, ctx) = setup_test_env()?;

    // Create and add files
    let file1 = dir.path().join("keep.txt");
    let file2 = dir.path().join("remove.txt");

    fs::write(&file1, "keep this")?;
    fs::write(&file2, "remove this")?;

    let paths = vec![
        file1.to_string_lossy().to_string(),
        file2.to_string_lossy().to_string(),
    ];
    commands::add::execute(&ctx, &paths, false)?;
    commands::commit::execute(&ctx, "Add files", false)?;

    // Remove file2 from tracking (--cached keeps file on disk)
    let rm_paths = vec![file2.to_string_lossy().to_string()];
    commands::rm::execute(&ctx, &rm_paths, true, false)?;

    // File should still exist on disk
    assert!(file2.exists());

    // But not in index
    let index = dotman::storage::index::Index::load(&ctx.repo_path.join("index.bin"))?;
    assert!(index.get_entry(&file1).is_some());
    assert!(index.get_entry(&file2).is_none());

    Ok(())
}

#[test]
fn test_ignore_patterns_workflow() -> Result<()> {
    let (dir, mut ctx) = setup_test_env()?;

    // Update config with ignore patterns
    ctx.config.tracking.ignore_patterns = vec![
        "*.log".to_string(),
        "*.tmp".to_string(),
        "cache/".to_string(),
        ".DS_Store".to_string(),
    ];
    ctx.config.save(&ctx.config_path)?;

    // Create files - some should be ignored
    let good_file = dir.path().join("important.txt");
    let log_file = dir.path().join("debug.log");
    let tmp_file = dir.path().join("temp.tmp");
    let cache_dir = dir.path().join("cache");
    let cache_file = cache_dir.join("cached.dat");

    fs::write(&good_file, "important")?;
    fs::write(&log_file, "log data")?;
    fs::write(&tmp_file, "temporary")?;
    fs::create_dir_all(&cache_dir)?;
    fs::write(&cache_file, "cached")?;

    // Try to add directory (should skip ignored files)
    let paths = vec![dir.path().to_string_lossy().to_string()];
    commands::add::execute(&ctx, &paths, false)?;

    // Check what was added
    let index = dotman::storage::index::Index::load(&ctx.repo_path.join("index.bin"))?;

    // Should have added good_file and other non-ignored files
    // But not log, tmp, or cache files
    // Check for specific files we created
    let has_important = index
        .entries
        .keys()
        .any(|p| p.file_name() == Some(std::ffi::OsStr::new("important.txt")));
    let has_debug_log = index
        .entries
        .keys()
        .any(|p| p.file_name() == Some(std::ffi::OsStr::new("debug.log")));
    let has_temp_tmp = index
        .entries
        .keys()
        .any(|p| p.file_name() == Some(std::ffi::OsStr::new("temp.tmp")));
    let has_cached_dat = index
        .entries
        .keys()
        .any(|p| p.file_name() == Some(std::ffi::OsStr::new("cached.dat")));

    assert!(has_important, "important.txt should be added");
    assert!(!has_debug_log, "debug.log should be ignored");
    assert!(!has_temp_tmp, "temp.tmp should be ignored");
    assert!(!has_cached_dat, "cached.dat in cache/ should be ignored");

    Ok(())
}

#[test]
fn test_concurrent_operations() -> Result<()> {
    use std::sync::Arc;
    use std::thread;

    let (dir, ctx) = setup_test_env()?;
    let ctx = Arc::new(ctx);

    // Create many files
    for i in 0..100 {
        let file = dir.path().join(format!("file_{}.txt", i));
        fs::write(&file, format!("content {}", i))?;
    }

    // Spawn threads to add files concurrently
    let handles: Vec<_> = (0..10)
        .map(|thread_id| {
            let ctx_clone = ctx.clone();
            let dir_path = dir.path().to_path_buf();

            thread::spawn(move || {
                let mut paths = Vec::new();
                for i in (thread_id * 10)..((thread_id + 1) * 10) {
                    let file = dir_path.join(format!("file_{}.txt", i));
                    paths.push(file.to_string_lossy().to_string());
                }
                commands::add::execute(&ctx_clone, &paths, false).unwrap();
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all files were added
    let index = dotman::storage::index::Index::load(&ctx.repo_path.join("index.bin"))?;
    assert_eq!(index.entries.len(), 100);

    Ok(())
}

#[test]
fn test_large_scale_operations() -> Result<()> {
    let (dir, ctx) = setup_test_env()?;

    // Create 1000 files
    let test_dir = dir.path().join("large_test");
    fs::create_dir_all(&test_dir)?;

    for i in 0..1000 {
        let file = test_dir.join(format!("file_{:04}.txt", i));
        fs::write(
            &file,
            format!("This is file number {} with some content", i),
        )?;
    }

    // Add all files
    let paths = vec![test_dir.to_string_lossy().to_string()];
    let start = std::time::Instant::now();
    commands::add::execute(&ctx, &paths, false)?;
    let add_time = start.elapsed();

    println!("Added 1000 files in {:?}", add_time);

    // Commit
    let start = std::time::Instant::now();
    commands::commit::execute(&ctx, "Large commit with 1000 files", false)?;
    let commit_time = start.elapsed();

    println!("Committed 1000 files in {:?}", commit_time);

    // Status check
    let start = std::time::Instant::now();
    commands::status::execute(&ctx, false)?;
    let status_time = start.elapsed();

    println!("Status check on 1000 files in {:?}", status_time);

    // Verify performance is reasonable
    assert!(
        add_time.as_secs() < 10,
        "Add should complete in < 10 seconds"
    );
    assert!(
        commit_time.as_secs() < 10,
        "Commit should complete in < 10 seconds"
    );
    assert!(
        status_time.as_secs() < 5,
        "Status should complete in < 5 seconds"
    );

    Ok(())
}

#[test]
fn test_binary_file_handling() -> Result<()> {
    let (dir, ctx) = setup_test_env()?;

    // Create binary file
    let binary_file = dir.path().join("binary.dat");
    let binary_content: Vec<u8> = (0..=255).collect();
    fs::write(&binary_file, &binary_content)?;

    // Create text file with special characters
    let text_file = dir.path().join("special.txt");
    fs::write(&text_file, "Special chars: 你好 مرحبا  🚀 \n\t\r")?;

    // Add files
    let paths = vec![
        binary_file.to_string_lossy().to_string(),
        text_file.to_string_lossy().to_string(),
    ];
    commands::add::execute(&ctx, &paths, false)?;

    // Commit
    commands::commit::execute(&ctx, "Binary and special character files", false)?;

    // Modify files
    fs::write(&binary_file, &binary_content[..128])?;
    fs::write(&text_file, "Modified: 世界 שלום 🌍")?;

    // Check status detects changes
    commands::status::execute(&ctx, false)?;

    Ok(())
}
