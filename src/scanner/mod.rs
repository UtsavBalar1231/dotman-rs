/// Directory trie for O(depth) directory role lookup during filesystem traversal.
pub mod dir_trie;

/// Untracked file discovery using trie-based directory filtering.
pub mod untracked;

pub use dir_trie::{DirTrie, DirectoryRole};
pub use untracked::find_untracked_files;
