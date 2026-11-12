use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static SERIAL: AtomicU64 = AtomicU64::new(0);

pub fn root() -> Result<PathBuf, String> {
    directories::ProjectDirs::from("", "", "starter-bevy")
        .map(|p| p.cache_dir().join("geo"))
        .ok_or("System cache directory unavailable".into())
}
pub fn id(value: &impl serde::Serialize) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}
pub struct Staging {
    pub path: PathBuf,
}
impl Staging {
    pub fn new(parent: &Path) -> Result<Self, String> {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let path = parent.join(format!(
            ".staging-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).map_err(|e| e.to_string())?;
        Ok(Self { path })
    }
    pub fn publish(&self, target: &Path) -> Result<(), String> {
        // Per-package lock held by the caller also protects replacement of corrupt entries.
        if target.exists() {
            fs::remove_dir_all(target).map_err(|e| e.to_string())?;
        }
        fs::rename(&self.path, target).map_err(|e| e.to_string())
    }
}
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
pub struct Lock {
    file: fs::File,
}
impl Lock {
    pub fn acquire(path: &Path, job: &super::Job) -> Result<Self, String> {
        fs::create_dir_all(path.parent().ok_or("Invalid cache lock path")?)
            .map_err(|e| e.to_string())?;
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        loop {
            job.check()?;
            match file.try_lock() {
                Ok(()) => return Ok(Self { file }),
                Err(std::fs::TryLockError::WouldBlock) => {
                    std::thread::sleep(std::time::Duration::from_millis(100))
                }
                Err(e) => return Err(e.to_string()),
            }
        }
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid cache path")?;
    let stage = Staging::new(parent)?;
    let temp = stage.path.join("file");
    let mut f = fs::File::create(&temp).map_err(|e| e.to_string())?;
    f.write_all(bytes)
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}
