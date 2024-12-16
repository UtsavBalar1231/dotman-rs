pub mod config;
pub mod errors;
mod file_manager;
mod hasher;

/// Checks if a path should be ignored based on `.git`-related patterns.
///
/// # Examples
///
/// ```rust
/// use std::path::Path;
/// use dotman_rs::is_git_related;
///
/// assert!(is_git_related(Path::new(".git")));
/// assert!(is_git_related(Path::new(".git/objects")));
/// assert!(is_git_related(Path::new(".gitignore")));
/// assert!(is_git_related(Path::new(".test/.gitattributes")));
/// assert!(is_git_related(Path::new("/test/.github")));
///
/// assert!(!is_git_related(Path::new("file.txt")));
/// assert!(!is_git_related(Path::new(".gihtubub")));
/// ```
pub fn is_git_related(path: &std::path::Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map_or(false, |s| s.starts_with(".git"))
    })
}
