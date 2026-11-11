use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::thread;

use eframe::egui::Context;
use starter_core::Document;

pub enum FileMessage {
    Opened(Option<Result<Document, String>>),
    Saved(Option<Result<(), String>>),
}

pub fn open(sender: Sender<FileMessage>, context: Context) {
    thread::spawn(move || {
        let result = pollster::block_on(async {
            let file = rfd::AsyncFileDialog::new()
                .set_title("Open starter document")
                .add_filter("JSON document", &["json"])
                .pick_file()
                .await?;
            Some(
                std::fs::read(file.path())
                    .map_err(|error| error.to_string())
                    .and_then(|bytes| decode_document(&bytes)),
            )
        });
        sender.send(FileMessage::Opened(result)).ok();
        context.request_repaint();
    });
}

fn decode_document(bytes: &[u8]) -> Result<Document, String> {
    let document: Document = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    document.validate().map_err(|error| error.to_string())?;
    Ok(document)
}

pub fn save(document: Document, sender: Sender<FileMessage>, context: Context) {
    thread::spawn(move || {
        let result = pollster::block_on(async move {
            let bytes = match serde_json::to_vec_pretty(&document) {
                Ok(bytes) => bytes,
                Err(error) => return Some(Err(error.to_string())),
            };
            let file = rfd::AsyncFileDialog::new()
                .set_title("Save starter document")
                .set_file_name("scene.json")
                .add_filter("JSON document", &["json"])
                .save_file()
                .await?;
            Some(write_atomic(file.path(), &bytes).map_err(|error| error.to_string()))
        });
        sender.send(FileMessage::Saved(result)).ok();
        context.request_repaint();
    });
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let (temporary, mut file) = create_temporary(path)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        sync_parent(path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn create_temporary(path: &Path) -> io::Result<(PathBuf, File)> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    loop {
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let candidate = path.with_file_name(format!(".{name}.{}.{unique}.tmp", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_json_and_unsupported_schema_are_reported() {
        assert!(decode_document(b"not json").is_err());
        let mut value = serde_json::to_value(Document::empty()).unwrap();
        value["schema_version"] = 99.into();
        assert!(decode_document(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn atomic_write_replaces_the_file_and_leaves_no_temporary() {
        let directory = test_directory("replace");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("scene.json");
        fs::write(&path, b"old").unwrap();

        write_atomic(&path, b"new").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert!(temporary_files(&directory).is_empty());
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn concurrent_writes_use_unique_temporary_files() {
        let directory = test_directory("concurrent");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("scene.json");

        let writers: Vec<_> = [b"first".as_slice(), b"second".as_slice()]
            .into_iter()
            .map(|bytes| {
                let path = path.clone();
                std::thread::spawn(move || write_atomic(&path, bytes).unwrap())
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }

        let saved = fs::read(&path).unwrap();
        assert!(saved == b"first" || saved == b"second");
        assert!(temporary_files(&directory).is_empty());
        fs::remove_dir_all(&directory).unwrap();
    }

    fn test_directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("starter-atomic-save-{}-{name}", std::process::id()))
    }

    fn temporary_files(directory: &Path) -> Vec<PathBuf> {
        fs::read_dir(directory)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "tmp"))
            .collect()
    }
}
