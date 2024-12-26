use fs_extra::{dir, file};
use std::{fs, io, path::Path};

/// Removes files and directories recursively.
///
/// # Errors
///
/// Returns an error if any part of the removal process fails.
pub fn fs_remove_recursive<P>(path: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    if path.exists() {
        if path.is_dir() {
            // Remove the directory and its contents
            dir::remove(path).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        } else if path.is_file() {
            // Remove a single file
            file::remove(path).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Path is neither a file nor a directory",
            ));
        }
    }

    Ok(())
}

/// Copies a file or directory recursively.
///
/// This function determines whether the source path is a file or a directory and handles the copying process accordingly.
///
/// # Errors
///
/// Returns an error if any part of the copy process fails.
pub fn fs_copy_recursive<P>(src: P, dst: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let src_path = src.as_ref();
    let dst_path = dst.as_ref();

    if !src_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Source path not found: {}", src_path.display()),
        ));
    }

    if src_path.is_file() {
        // Copy a single file
        if dst_path.is_dir() {
            let dest_file_path = dst_path.join(src_path.file_name().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Destination is a directory but source file has no name",
                )
            })?);
            fs::copy(src_path, &dest_file_path).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to copy file: {}", e.to_string()),
                )
            })?;
        } else {
            fs::copy(src_path, dst_path).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to copy file: {}", e.to_string()),
                )
            })?;
        }
    } else if src_path.is_dir() {
        // Copy a directory and its contents
        if !dst_path.exists() {
            fs::create_dir_all(dst_path)?;
        }

        for entry in fs::read_dir(src_path)? {
            let entry = entry?;
            let entry_path = entry.path();
            let dest_entry_path = dst_path.join(entry.file_name());

            if entry_path.is_dir() {
                fs_copy_recursive(&entry_path, &dest_entry_path)?;
            } else {
                fs::copy(&entry_path, &dest_entry_path).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("Failed to copy file: {}", e.to_string()),
                    )
                })?;
            }
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Source path is neither file nor directory",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    #[test]
    fn test_fs_copy_recursive() {
        let src = PathBuf::from("test_src");
        let dst = PathBuf::from("test_dst");

        // Setup
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("file1.txt"), "Hello, World!").unwrap();
        fs::create_dir_all(src.join("subdir")).unwrap();
        fs::write(src.join("subdir/file2.txt"), "Another file!").unwrap();

        // Perform copy
        fs_copy_recursive(&src, &dst).unwrap();

        // Verify
        assert!(dst.join("file1.txt").exists());
        assert!(dst.join("subdir/file2.txt").exists());

        // Cleanup
        fs_remove_recursive(&src).unwrap();
        fs_remove_recursive(&dst).unwrap();
    }

    #[test]
    fn test_fs_remove_recursive() {
        let path = PathBuf::from("test_remove");

        // Setup
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("file.txt"), "To be deleted.").unwrap();

        // Perform removal
        fs_remove_recursive(&path).unwrap();

        // Verify
        assert!(!path.exists());
    }
}
