use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{broadcast, Mutex};

use crate::server::application::IngestLogEvent;

const BROADCAST_CAPACITY: usize = 256;

pub struct JobInner {
    pub buffered: Vec<IngestLogEvent>,
    pub finished: bool,
}

pub struct Job {
    pub inner: Mutex<JobInner>,
    pub sender: broadcast::Sender<JobMessage>,
}

#[derive(Debug, Clone)]
pub enum JobMessage {
    Event(IngestLogEvent),
    Done,
}

impl Job {
    fn new() -> Arc<Self> {
        let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
        Arc::new(Self {
            inner: Mutex::new(JobInner {
                buffered: Vec::new(),
                finished: false,
            }),
            sender,
        })
    }

    /// Lock + push to buffer + broadcast in a single critical section so that
    /// SSE subscribers can take a consistent snapshot of `buffered` and
    /// subscribe to the broadcaster without missing or duplicating events.
    pub async fn emit(&self, event: IngestLogEvent) {
        let mut inner = self.inner.lock().await;
        inner.buffered.push(event.clone());
        let _ = self.sender.send(JobMessage::Event(event));
    }

    pub async fn finish(&self) {
        let mut inner = self.inner.lock().await;
        inner.finished = true;
        let _ = self.sender.send(JobMessage::Done);
    }
}

pub struct JobRegistry {
    jobs: Mutex<HashMap<String, Arc<Job>>>,
}

impl JobRegistry {
    pub fn new() -> Self {
        Self {
            jobs: Mutex::new(HashMap::new()),
        }
    }

    pub async fn create(&self) -> (String, Arc<Job>) {
        let id = generate_id();
        let job = Job::new();
        self.jobs.lock().await.insert(id.clone(), job.clone());
        (id, job)
    }

    pub async fn get(&self, id: &str) -> Option<Arc<Job>> {
        self.jobs.lock().await.get(id).cloned()
    }
}

impl Default for JobRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos)
}
