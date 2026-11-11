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
            Some(std::fs::write(file.path(), bytes).map_err(|error| error.to_string()))
        });
        sender.send(FileMessage::Saved(result)).ok();
        context.request_repaint();
    });
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
}
