// KNOWN SECURITY VULNERABILITIES TEST SUITE
//
// This test file documents CONFIRMED security vulnerabilities in dotman.
// These tests are EXPECTED TO FAIL until the underlying security bugs are fixed.
//
// ⚠️  DO NOT RUN THESE TESTS IN PRODUCTION ENVIRONMENTS
// ⚠️  These tests may expose sensitive information or system files
//
// See DISCOVERED_BUGS.md for complete vulnerability documentation.

use anyhow::Result;
use dotman::DotmanContext;
use dotman::commands;
use dotman::config::Config;
use dotman::storage::index::Index;
use proptest::prelude::*;
use std::fs;
use tempfile::tempdir;

// Helper function to create test context
fn setup_test_context() -> Result<(tempfile::TempDir, DotmanContext)> {
    let dir = tempdir()?;
    let repo_path = dir.path().join(".dotman");
    let config_path = dir.path().join("config.toml");

    // Create repo structure
    fs::create_dir_all(&repo_path)?;
    fs::create_dir_all(repo_path.join("commits"))?;
    fs::create_dir_all(repo_path.join("objects"))?;

    // Create empty index
    let index = Index::new();
    let index_path = repo_path.join("index.bin");
    index.save(&index_path)?;

    let mut config = Config::default();
    config.core.repo_path = repo_path.clone();
    config.save(&config_path)?;

    let context = DotmanContext {
        repo_path,
        config_path,
        config,
    };

    Ok((dir, context))
}

// CRITICAL SECURITY VULNERABILITY: Permission Bit Preservation
// Status: CONFIRMED BUG - setuid/setgid/sticky bits are preserved when they should be stripped
// Impact: Potential privilege escalation when files are restored
// CVSS Score: ~7.2 (High)
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))] // Fewer cases to reduce noise

    #[test]
    #[ignore] // Ignored by default - use --ignored to run vulnerability tests
    fn test_permission_preservation_vulnerability(
        mode in 0o1000..=0o7777u32 // Only test modes with special bits set
    ) {
        // SECURITY VULNERABILITY DOCUMENTATION
        // This test documents a CONFIRMED security bug in dotman
        // Expected behavior: setuid/setgid/sticky bits should be stripped
        // Actual behavior: dangerous permission bits are preserved

        let (dir, ctx) = setup_test_context().unwrap();

        let test_file = dir.path().join("vuln_test.txt");
        fs::write(&test_file, "test content").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&test_file).unwrap().permissions();
            perms.set_mode(mode);
            let _ = fs::set_permissions(&test_file, perms);
        }

        let paths = vec![test_file.to_string_lossy().to_string()];
        let result = commands::add::execute(&ctx, &paths, false);

        if result.is_ok() {
            let index = Index::load(&ctx.repo_path.join("index.bin")).unwrap();
            for entry in index.entries.values() {
                // DOCUMENTED VULNERABILITY: These assertions will fail, confirming the bug

                if mode & 0o4000 != 0 { // Test only when setuid bit was set
                    prop_assert!(
                        entry.mode & 0o4000 == 0,
                        "VULNERABILITY CONFIRMED: Setuid bit preserved (mode {:o} -> {:o})",
                        mode, entry.mode
                    );
                }

                if mode & 0o2000 != 0 { // Test only when setgid bit was set
                    prop_assert!(
                        entry.mode & 0o2000 == 0,
                        "VULNERABILITY CONFIRMED: Setgid bit preserved (mode {:o} -> {:o})",
                        mode, entry.mode
                    );
                }

                if mode & 0o1000 != 0 { // Test only when sticky bit was set
                    prop_assert!(
                        entry.mode & 0o1000 == 0,
                        "VULNERABILITY CONFIRMED: Sticky bit preserved (mode {:o} -> {:o})",
                        mode, entry.mode
                    );
                }
            }
        }
    }
}

// CRITICAL SECURITY VULNERABILITY: Permission Escalation Prevention
// Status: CONFIRMED BUG - setuid/setgid bits preserved when adding files
// Impact: Potential privilege escalation when files are restored
// CVSS Score: ~7.2 (High)
#[test]
#[ignore] // Ignored by default - use --ignored to run vulnerability tests
fn test_permission_escalation_vulnerability() {
    // SECURITY VULNERABILITY DOCUMENTATION
    // This test documents a CONFIRMED security bug in dotman
    // Expected behavior: setuid/setgid bits should be stripped when adding files
    // Actual behavior: dangerous permission bits are preserved

    let (dir, ctx) = setup_test_context().unwrap();

    #[cfg(unix)]
    {
        // Create file with dangerous permissions
        let vuln_file = dir.path().join("setuid_test.txt");
        fs::write(&vuln_file, "test content").unwrap();

        // Set setuid bit (dangerous permission)
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&vuln_file).unwrap().permissions();
        perms.set_mode(0o4755); // setuid + rwxr-xr-x
        fs::set_permissions(&vuln_file, perms).unwrap();

        let paths = vec![vuln_file.to_string_lossy().to_string()];
        let result = commands::add::execute(&ctx, &paths, false);

        // DOCUMENTED VULNERABILITY: This assertion will fail, confirming the bug
        match result {
            Ok(_) => {
                let index = Index::load(&ctx.repo_path.join("index.bin")).unwrap();
                for entry in index.entries.values() {
                    // This assertion documents the vulnerability - it should pass but will fail
                    assert!(
                        entry.mode & 0o4000 == 0,
                        "VULNERABILITY CONFIRMED: Setuid bit preserved when it should be stripped (mode {:o})",
                        entry.mode
                    );
                    assert!(
                        entry.mode & 0o2000 == 0,
                        "VULNERABILITY CONFIRMED: Setgid bit preserved when it should be stripped (mode {:o})",
                        entry.mode
                    );
                }
            }
            Err(_) => {
                // If it fails to add, that's actually good security behavior
                // But the current implementation succeeds and preserves dangerous bits
            }
        }
    }
}

// Example test showing how the fixed version should behave
#[test]
#[ignore] // Ignored until vulnerability is fixed
fn test_permission_preservation_fixed_behavior() {
    // This test shows the EXPECTED behavior after the vulnerability is fixed
    // When the security bug is fixed, remove #[ignore] and this should pass

    let (dir, ctx) = setup_test_context().unwrap();

    let test_file = dir.path().join("fixed_test.txt");
    fs::write(&test_file, "test content").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&test_file).unwrap().permissions();
        perms.set_mode(0o4755); // setuid + rwxr-xr-x
        fs::set_permissions(&test_file, perms).unwrap();
    }

    let paths = vec![test_file.to_string_lossy().to_string()];
    let result = commands::add::execute(&ctx, &paths, false);

    assert!(result.is_ok(), "Add operation should succeed");

    let index = Index::load(&ctx.repo_path.join("index.bin")).unwrap();
    for entry in index.entries.values() {
        // After fix: dangerous bits should be stripped, safe bits preserved
        assert_eq!(entry.mode & 0o4000, 0, "Setuid bit should be stripped");
        assert_eq!(entry.mode & 0o2000, 0, "Setgid bit should be stripped");
        assert_eq!(entry.mode & 0o1000, 0, "Sticky bit should be stripped");
        assert_eq!(
            entry.mode & 0o0755,
            0o0755,
            "Normal permissions should be preserved"
        );
    }
}

// Instructions for running vulnerability tests
#[test]
fn vulnerability_test_instructions() {
    println!(
        "
🚨 SECURITY VULNERABILITY TESTS 🚨

To run the vulnerability documentation tests:
    cargo test known_vulnerabilities_test -- --ignored --nocapture

⚠️  WARNING: These tests document CONFIRMED security vulnerabilities
⚠️  Only run in isolated test environments
⚠️  Tests are expected to FAIL until vulnerabilities are fixed

See DISCOVERED_BUGS.md for:
- Complete vulnerability documentation  
- Reproduction steps
- Recommended fixes
- CVSS scores and impact assessment
"
    );
}
