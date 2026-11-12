use super::{Job, coverage::Tile};
use std::{
    collections::BTreeMap,
    io::Read,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

pub struct Download {
    client: reqwest::blocking::Client,
    pub parallelism: usize,
}
impl Download {
    pub fn new(parallelism: usize) -> Result<Self, String> {
        Self::with_timeout(parallelism, Duration::from_secs(25))
    }
    pub(super) fn with_timeout(parallelism: usize, timeout: Duration) -> Result<Self, String> {
        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .timeout(timeout)
                .connect_timeout(Duration::from_secs(10))
                .user_agent("starter-bevy/0.1 geographic-package-builder")
                .build()
                .map_err(|_| "Cannot initialize HTTP client")?,
            parallelism,
        })
    }
    pub fn get(&self, url: &str, token: Option<&str>, job: &Job) -> Result<Vec<u8>, String> {
        let mut last = "Source unavailable".to_owned();
        for attempt in 0..3 {
            job.check()?;
            let mut request = self.client.get(url);
            if let Some(token) = token {
                request = request.bearer_auth(token);
            }
            match request.send() {
                Ok(response) => {
                    let status = response.status();
                    if status == reqwest::StatusCode::NOT_FOUND
                        || status == reqwest::StatusCode::NO_CONTENT
                    {
                        return Err("Missing geographic tile coverage".into());
                    }
                    if status.is_success() {
                        let mut bytes = Vec::new();
                        response
                            .take(8 * 1024 * 1024 + 1)
                            .read_to_end(&mut bytes)
                            .map_err(|_| "Interrupted HTTP response")?;
                        job.check()?;
                        if bytes.is_empty() || bytes.len() > 8 * 1024 * 1024 {
                            return Err("Invalid tile response size".into());
                        }
                        return Ok(bytes);
                    }
                    last = format!("Source returned HTTP {status}");
                    if !status.is_server_error() && status != reqwest::StatusCode::TOO_MANY_REQUESTS
                    {
                        return Err(last);
                    }
                }
                Err(error) => {
                    last = if error.is_timeout() {
                        "Source request timed out"
                    } else {
                        "Source unavailable"
                    }
                    .into()
                }
            }
            if attempt < 2 {
                for _ in 0..(5 << attempt) {
                    job.check()?;
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
        Err(last)
    }
    pub fn tiles(
        &self,
        template: &str,
        token: Option<&str>,
        tiles: &[Tile],
        cache: &std::path::Path,
        job: &Job,
        phase: &str,
    ) -> Result<BTreeMap<Tile, Vec<u8>>, String> {
        let next = AtomicUsize::new(0);
        let done = AtomicUsize::new(0);
        let result = Mutex::new(BTreeMap::new());
        let error = Mutex::new(None);
        std::thread::scope(|scope| {
            for _ in 0..self.parallelism.min(tiles.len()) {
                scope.spawn(|| {
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= tiles.len() || error.lock().unwrap().is_some() {
                            break;
                        }
                        let t = tiles[i];
                        let path = cache.join(format!("{}/{}/{}.tile", t.z, t.x, t.y));
                        let url = template
                            .replace("{z}", &t.z.to_string())
                            .replace("{x}", &t.x.to_string())
                            .replace("{y}", &t.y.to_string());
                        let value = job.check().and_then(|_| {
                            if let Ok(bytes) =
                                crate::map::source::read_limited(&path, 8 * 1024 * 1024)
                            {
                                return Ok(bytes);
                            }
                            let bytes = self.get(&url, token, job)?;
                            super::cache::write_atomic(&path, &bytes)?;
                            Ok(bytes)
                        });
                        match value {
                            Ok(bytes) => {
                                result.lock().unwrap().insert(t, bytes);
                                let mut progress = job.progress.lock().unwrap();
                                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                                *progress = format!("{phase} · {n} / {} tiles", tiles.len());
                            }
                            Err(e) => {
                                *error.lock().unwrap() = Some(e);
                                break;
                            }
                        }
                    }
                });
            }
        });
        if let Some(e) = error.into_inner().unwrap() {
            return Err(e);
        }
        Ok(result.into_inner().unwrap())
    }
}
