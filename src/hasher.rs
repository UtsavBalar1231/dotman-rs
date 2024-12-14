use digest::DynDigest;
use std::{
    collections::HashMap,
    fmt, fs,
    io::{self, Read},
    marker,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    thread,
};

struct HashBox(Box<[u8]>);

impl fmt::LowerHex for HashBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .for_each(|byte| write!(f, "{:02x}", byte).expect("Failed to write to string"));
        Ok(())
    }
}

impl fmt::Display for HashBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .for_each(|byte| write!(f, "{:02x}", byte).expect("Failed to write to string"));
        Ok(())
    }
}

pub fn list_dir_files<P>(p: P) -> Result<Vec<PathBuf>, io::Error>
where
    P: AsRef<Path>,
{
    Ok(walkdir::WalkDir::new(p)
        .into_iter()
        .filter_map(|file| file.ok())
        .filter(|entry| !entry.path().components().any(|c| c.as_os_str() == ".git"))
        .filter_map(|entry| {
            entry.metadata().map_or(None, |m| {
                if m.is_file() {
                    Some(entry.into_path())
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>())
}

pub fn get_file_hash<Hasher>(
    path: &PathBuf,
    hash: &mut Hasher,
    cache: &mut HashMap<PathBuf, String>,
) -> Result<String, io::Error>
where
    Hasher: DynDigest + Clone,
{
    if let Some(cached_hash) = cache.get(path) {
        if let Ok(metadata) = fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(cached_modified) = metadata.modified() {
                    if modified == cached_modified {
                        return Ok(cached_hash.clone());
                    }
                }
            }
        }
    }
    let mut file = fs::File::open(path)?;
    let mut buf = [0u8; 4096];

    loop {
        let i = file.read(&mut buf)?;
        hash.update(&buf[..i]);

        if i == 0 {
            let final_hash = HashBox(hash.finalize_reset()).to_string();
            cache.insert(path.to_path_buf(), final_hash.clone());
            return Ok(final_hash);
        }
    }
}

pub fn get_files_hash<Hasher>(
    files: &[PathBuf],
    hash: &mut Hasher,
    cache: &mut HashMap<PathBuf, String>,
) -> Result<String, io::Error>
where
    Hasher: DynDigest + marker::Send + Clone,
{
    if files.is_empty() {
        return Ok(String::new());
    }

    let threads = thread::available_parallelism()
        .unwrap_or(NonZeroUsize::MIN)
        .get();

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let results: Result<Vec<_>, _> = files
        .iter()
        .map(|file| pool.install(|| get_file_hash(file, hash, cache)))
        .collect();

    results?;

    let final_hash = HashBox(hash.finalize_reset()).to_string();

    Ok(final_hash)
}

pub fn get_complete_dir_hash<Hasher>(
    dir_path: &PathBuf,
    hash: &mut Hasher,
    cache: &mut HashMap<PathBuf, String>,
) -> Result<String, io::Error>
where
    Hasher: DynDigest + Clone + marker::Send,
{
    let dirs = list_dir_files(dir_path)?;
    let mut paths = Vec::with_capacity(dirs.len());

    dirs.iter()
        .for_each(|dir| paths.append(&mut list_dir_files(dir).unwrap()));

    get_files_hash(&paths, hash, cache)
}
