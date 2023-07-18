use crate::*;
use std::collections::BTreeSet;

pub(crate) struct DynamicRegistry {
    metrics: RwLock<BTreeSet<DynamicEntry>>,
}

impl DynamicRegistry {
    pub(crate) const fn new() -> Self {
        Self {
            metrics: RwLock::new(BTreeSet::new()),
        }
    }

    pub(crate) fn register(&self, entry: DynamicEntry) {
        let mut metrics = self.metrics.write();
        metrics.insert(entry);
    }

    pub(crate) fn deregister(&self, metric: Arc<dyn Metric>) {
        let mut metrics = self.metrics.write();

        metrics.retain(|entry| {
            // compares only the data addresses of the wide pointers
            Arc::as_ptr(&entry.metric) as *const () != Arc::as_ptr(&metric) as *const ()
        });
    }

    pub(crate) fn read(&self) -> RwLockReadGuard<BTreeSet<DynamicEntry>> {
        self.metrics.read()
    }
}
