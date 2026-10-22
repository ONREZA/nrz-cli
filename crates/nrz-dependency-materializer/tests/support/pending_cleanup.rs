use super::PendingErofsToolCleanup;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

pub(super) type Retained = Arc<Mutex<Vec<Box<dyn Send>>>>;

pub(super) struct PendingCleanup {
    pub(super) retained: Retained,
    pub(super) handoff_closed: Arc<AtomicUsize>,
}

impl std::fmt::Debug for PendingCleanup {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PendingCleanup")
    }
}

impl std::fmt::Display for PendingCleanup {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("worker cleanup pending")
    }
}

impl std::error::Error for PendingCleanup {}

impl PendingErofsToolCleanup for PendingCleanup {
    fn retain(&self, resource: Box<dyn Send>) {
        self.retained.lock().unwrap().push(resource);
    }
}

impl Drop for PendingCleanup {
    fn drop(&mut self) {
        self.handoff_closed.fetch_add(1, Ordering::Relaxed);
    }
}
