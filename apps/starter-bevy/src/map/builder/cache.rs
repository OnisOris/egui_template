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
    _guard: fs::File,
}
impl Staging {
    pub fn new(parent: &Path) -> Result<Self, String> {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        cleanup(parent)?;
        let path = parent.join(format!(
            ".staging-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).map_err(|e| e.to_string())?;
        let guard = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path.join(".active"))
            .map_err(|e| e.to_string())?;
        guard.lock().map_err(|e| e.to_string())?;
        Ok(Self {
            path,
            _guard: guard,
        })
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

/// Only old, unlocked staging directories are abandoned; active writers keep their lock.
fn cleanup(parent: &Path) -> Result<(), String> {
    for entry in fs::read_dir(parent).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_name().to_string_lossy().starts_with(".staging-") {
            continue;
        }
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_dir()
            || metadata
                .modified()
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_none_or(|age| age.as_secs() < 60)
        {
            continue;
        }
        let guard = fs::OpenOptions::new()
            .write(true)
            .open(entry.path().join(".active"));
        match guard {
            Ok(file) if file.try_lock().is_ok() => {
                let _ = fs::remove_dir_all(entry.path());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let _ = fs::remove_dir_all(entry.path());
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locks_release_and_old_staging_cleanup_preserves_active_writers() {
        let root = tempfile::tempdir().unwrap();
        let job = super::super::network_tests::job();
        let lock_path = root.path().join("lock");
        let guard = Lock::acquire(&lock_path, &job).unwrap();
        let other = fs::OpenOptions::new().write(true).open(&lock_path).unwrap();
        assert!(matches!(
            other.try_lock(),
            Err(fs::TryLockError::WouldBlock)
        ));
        job.cancelled.store(true, Ordering::Relaxed);
        assert!(Lock::acquire(&lock_path, &job).is_err());
        drop(guard);
        other.try_lock().unwrap();
        let active = Staging::new(root.path()).unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
        fs::File::open(&active.path)
            .unwrap()
            .set_modified(old)
            .unwrap();
        let abandoned = root.path().join(".staging-abandoned");
        fs::create_dir(&abandoned).unwrap();
        fs::File::create(abandoned.join(".active")).unwrap();
        fs::File::open(&abandoned)
            .unwrap()
            .set_modified(old)
            .unwrap();
        let _new = Staging::new(root.path()).unwrap();
        assert!(active.path.exists());
        assert!(!abandoned.exists());
    }
}
