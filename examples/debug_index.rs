use dotman::storage::index::Index;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let index_path = PathBuf::from("/tmp/status_test3/.dotman/index.bin");

    if !index_path.exists() {
        println!("Index file does not exist at: {}", index_path.display());
        return Ok(());
    }

    let index = Index::load(&index_path)?;

    println!("Index contains {} entries:", index.entries.len());
    for (path, entry) in &index.entries {
        println!(
            "  Path: {} (absolute: {})",
            path.display(),
            path.is_absolute()
        );
        if let Some(parent) = path.parent() {
            println!("    Parent: {}", parent.display());
        }
        println!("    Hash: {}", entry.hash);
        println!("    Size: {}", entry.size);
    }

    Ok(())
}
