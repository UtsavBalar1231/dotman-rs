use digest::DynDigest;
use std::{
    fmt,
    fmt::Write,
    fs, io,
    io::Read,
    marker,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    string, thread,
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

impl string::ToString for HashBox {
    fn to_string(&self) -> String {
        let mut hex_string = String::with_capacity(self.0.len() * 2);
        self.0.iter().for_each(|byte| {
            write!(hex_string, "{:02x}", byte).expect("Failed to write to string");
        });
        hex_string
    }
}

pub fn list_dir_files<P>(p: P) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    walkdir::WalkDir::new(p)
        .into_iter()
        .filter_map(|file| file.ok())
        .filter(|entry| {
            // Skip entries inside a .git folder
            !entry.path().components().any(|c| c.as_os_str() == ".git")
        })
        .filter(|normal_file| normal_file.metadata().unwrap().is_file())
        .map(|x| x.into_path())
        .collect::<Vec<PathBuf>>()
}

pub fn get_file_hash<Hasher, P>(path: P, hash: &mut Hasher) -> Result<String, io::Error>
where
    Hasher: DynDigest + Clone,
    P: AsRef<Path>,
{
    let mut file = fs::File::open(path)?;
    let mut buf = [0u8; 4096];

    loop {
        let i = file.read(&mut buf)?;
        hash.update(&buf[..i]);

        if i == 0 {
            let final_hash = HashBox(hash.finalize_reset()).to_string();
            return Ok(final_hash);
        }
    }
}

pub fn get_files_hash<Hasher, P>(files: &[P], hash: &mut Hasher) -> Result<String, io::Error>
where
    P: AsRef<Path> + marker::Sync,
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
        .unwrap();

    let mut jobs: Vec<_> = Vec::with_capacity(files.len());

    files.iter().for_each(|file| {
        jobs.push(pool.install(|| -> Result<(), io::Error> {
            let filehash = get_file_hash(file, hash)?;
            hash.update(filehash.as_bytes());
            Ok(())
        }))
    });

    let final_hash = HashBox(hash.finalize_reset()).to_string();

    Ok(final_hash)
}

pub fn get_complete_dir_hash<Hasher, P>(dir_path: P, hash: &mut Hasher) -> Result<String, io::Error>
where
    Hasher: DynDigest + Clone + marker::Send,
    P: AsRef<Path> + marker::Sync,
{
    let dirs = list_dir_files(&dir_path);
    let mut paths: Vec<PathBuf> = vec![];

    dirs.iter()
        .for_each(|dir| paths.append(&mut list_dir_files(dir)));

    get_files_hash(&paths, hash)
}
