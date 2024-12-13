use ron::{extensions::Extensions, ser::PrettyConfig};
use std::path::Path;
use std::{fs, io};

pub fn get_ron_formatter() -> PrettyConfig {
    PrettyConfig::new()
        .depth_limit(2)
        .extensions(Extensions::IMPLICIT_SOME)
}

pub fn copy_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    if src.is_dir() {
        if src.file_name().unwrap_or_default() == ".git" {
            return Ok(());
        }
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_entry = entry.path();
            let dst_entry = dst.join(entry.file_name());
            if src_entry.is_dir() {
                copy_recursive(&src_entry, &dst_entry)?;
            } else {
                let result = fs::copy(&src_entry, &dst_entry);
                if result.is_err() {
                    let error = result.unwrap_err();
                    if error.kind() == io::ErrorKind::PermissionDenied {
                        println!("Permission denied while copying: {}", src_entry.display());
                    } else {
                        println!("Failed to copy {}: {}", src_entry.display(), error);
                    }
                }
            }
        }
    } else {
        let result = fs::copy(src, dst);
        if result.is_err() {
            let error = result.unwrap_err();
            if error.kind() == io::ErrorKind::PermissionDenied {
                println!("Permission denied while copying: {}", src.display());
            } else {
                println!("Failed to copy {}: {}", src.display(), error);
            }
        }
    }
    Ok(())
}
