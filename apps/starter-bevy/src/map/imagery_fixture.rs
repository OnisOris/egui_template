//! Local multi-zoom XYZ server; never contacts public map services.
use super::builder::provider::ImageryProvider;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
pub struct Fixture {
    pub provider: ImageryProvider,
    pub requests: Arc<Mutex<Vec<String>>>,
    pub fail: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Fixture {
    pub fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let mut provider = super::sources::esri();
        provider.name = "Local licensed fixture".into();
        provider.url = format!(
            "http://{}/{{z}}/{{x}}/{{y}}",
            listener.local_addr().unwrap()
        );
        provider.allow_online_3d = true;
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let fail = Arc::new(AtomicBool::new(false));
        let (seen, stopped, failed) = (requests.clone(), stop.clone(), fail.clone());
        let thread = std::thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = [0; 4096];
                let Ok(n) = stream.read(&mut request) else {
                    continue;
                };
                let text = String::from_utf8_lossy(&request[..n]);
                let path = text.split_whitespace().nth(1).unwrap().to_owned();
                seen.lock().unwrap().push(path.clone());
                let ids: Vec<u32> = path
                    .trim_start_matches('/')
                    .split('/')
                    .map(|v| v.parse().unwrap())
                    .collect();
                let color = image::Rgb([(ids[0] * 11) as u8, ids[1] as u8, ids[2] as u8]);
                let mut bytes = std::io::Cursor::new(Vec::new());
                image::RgbImage::from_pixel(256, 256, color)
                    .write_to(&mut bytes, image::ImageFormat::Png)
                    .unwrap();
                let (status, body) = if failed.load(Ordering::Relaxed) {
                    (403, &[][..])
                } else {
                    (200, bytes.get_ref().as_slice())
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(body);
            }
        });
        Self {
            provider,
            requests,
            fail,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}
pub fn job() -> super::builder::Job {
    super::builder::Job {
        cancelled: Arc::new(AtomicBool::new(false)),
        progress: Arc::default(),
    }
}
