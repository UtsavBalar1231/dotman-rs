1. rayon wherever possible
2. Unit and Integration Tests
3. Documentation and Examples
4. Optimize File Copying
```rust
use fs_extra::dir::{copy as copy_dir, CopyOptions as DirCopyOptions};
use fs_extra::file::{copy as copy_file, CopyOptions as FileCopyOptions};

impl Config {
    fn copy_with_fs_extra(src: &Path, dst: &Path) -> io::Result<()> {
        if src.is_dir() {
            let mut options = DirCopyOptions::new();
            options.overwrite = true; // Overwrite existing files
            options.copy_inside = true; // Copy contents of the directory
            copy_dir(src, dst, &options).map_err(|err| {
                io::Error::new(io::ErrorKind::Other, format!("Failed to copy directory: {}", err))
            })?;
        } else if src.is_file() {
            let mut options = FileCopyOptions::new();
            options.overwrite = true; // Overwrite existing file
            copy_file(src, dst, &options).map_err(|err| {
                io::Error::new(io::ErrorKind::Other, format!("Failed to copy file: {}", err))
            })?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Source path does not exist: {}", src.display()),
            ));
        }
        Ok(())
    }
}

pub fn pull_config(&mut self, clean: bool) -> io::Result<()> {
    let backup_path = &self.dotconfigs_path.get_path();

    if !backup_path.exists() {
        fs::create_dir_all(backup_path)?;
    }

    for entry in &mut self.configs {
        let src_path = entry.path.clone();
        let current_hash = if src_path.is_dir() {
            let mut hasher = Sha1::new();
            hasher::get_complete_dir_hash(&src_path, &mut hasher).unwrap_or_default()
        } else if src_path.is_file() {
            let mut hasher = Sha1::new();
            hasher::get_file_hash(&src_path, &mut hasher).unwrap_or_default()
        } else {
            String::new()
        };

        if entry.hash != current_hash || clean {
            println!("Pulling: {}", entry.name);
            let dest_path = backup_path.join(&entry.name);
            Self::copy_with_fs_extra(&src_path, &dest_path)?;
            entry.hash = current_hash;
        } else {
            println!("No changes detected for: {}", entry.name);
        }
    }

    self.save_config()
}

pub fn push_config(&self, clean: bool) -> Result<(), ConfigError> {
    let backup_path = &self.dotconfigs_path.get_path();

    if !backup_path.exists() {
        fs::create_dir_all(backup_path)?;
    }

    for entry in &self.configs {
        let src_path = backup_path.join(&entry.name);
        let dst_path = entry.path.clone();

        if clean {
            if dst_path.exists() {
                if dst_path.is_dir() {
                    fs::remove_dir_all(&dst_path)?;
                } else {
                    fs::remove_file(&dst_path)?;
                }
            }
        }

        println!("Pushing: {}", entry.name);
        Self::copy_with_fs_extra(&src_path, &dst_path)?;
    }

    self.save_config()
}
```

5. Add github repo tracking support
