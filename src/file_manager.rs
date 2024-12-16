use rayon::prelude::*;
use std::{fs, io, path::Path};
use walkdir::WalkDir;

/// Recursively removes files and directories with optimized parallel processing.
///
/// # Errors
///
/// Returns an error if any part of the removal process fails.
pub fn fs_remove_recursive<P>(path: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    if path.is_dir() {
        // Collect all entries in the directory for parallel processing
        let entries: Vec<_> = fs::read_dir(path)?.filter_map(|e| e.ok()).collect();

        entries.par_iter().try_for_each(|entry| {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                fs_remove_recursive(&entry_path)
            } else {
                fs::remove_file(&entry_path)
            }
        })?;

        // Remove the directory itself after its contents are cleared
        fs::remove_dir(path)?;
    } else if path.is_file() {
        fs::remove_file(path)?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Path is neither a file nor a directory",
        ));
    }

    Ok(())
}

/// Copies a file or directory recursively.
///
/// # Panics
///
/// Panics if the source path is not a valid file or directory.
///
/// # Errors
///
/// This function will return an error if any part of the copy process fails.
pub fn fs_copy_recursive<P>(src: P, dst: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let src = src.as_ref();
    let dst = dst.as_ref();

    if src.is_dir() {
        fs::create_dir_all(dst)?;

        for entry in WalkDir::new(src)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| !e.path().components().any(|c| c.as_os_str() == ".git"))
        {
            let src_entry = entry.path();
            let dst_entry = dst.join(src_entry.strip_prefix(src).expect("Failed to strip prefix"));

            if src_entry.is_dir() {
                fs::create_dir_all(&dst_entry)?;
            } else if src_entry.is_file() {
                fs::copy(src_entry, &dst_entry)?;
            }
        }
    } else if src.is_file() {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Source is not a valid file or directory",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_file_manager_fs_copy_recursive_file() {
        let temp_dir = tempdir().unwrap();
        let src_path = temp_dir.path().join("test_file.txt");
        let dst_path = temp_dir.path().join("copy_test_file.txt");
        let mut file = File::create(&src_path).unwrap();
        writeln!(file, "test content").unwrap();
        fs_copy_recursive(&src_path, &dst_path).unwrap();
        assert!(dst_path.exists());
        let copied_content = fs::read_to_string(&dst_path).unwrap();
        assert_eq!(copied_content, "test content\n");
    }

    #[test]
    fn test_file_manager_fs_copy_recursive_dir() {
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("test_dir");
        let dst_dir = temp_dir.path().join("copy_test_dir");
        fs::create_dir_all(&src_dir).unwrap();
        let file_in_dir = src_dir.join("test_file.txt");
        let mut file = File::create(&file_in_dir).unwrap();
        writeln!(file, "test content").unwrap();
        fs_copy_recursive(&src_dir, &dst_dir).unwrap();
        assert!(dst_dir.exists());
        assert!(dst_dir.join("test_file.txt").exists());
        let copied_content = fs::read_to_string(dst_dir.join("test_file.txt")).unwrap();
        assert_eq!(copied_content, "test content\n");
    }

    #[test]
    fn test_file_manager_fs_remove_recursive_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        File::create(&file_path).unwrap();
        fs_remove_recursive(&file_path).unwrap();
        assert!(!file_path.exists());
    }

    #[test]
    fn test_file_manager_fs_remove_recursive_dir() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path().join("test_dir");
        fs::create_dir_all(&dir_path).unwrap();
        let file_in_dir = dir_path.join("test_file.txt");
        File::create(&file_in_dir).unwrap();
        fs_remove_recursive(&dir_path).unwrap();
        assert!(!dir_path.exists());
    }
}
