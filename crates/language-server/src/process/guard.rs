use dashmap::DashSet;

pub struct ProcessingGuard<'a> {
    set: &'a DashSet<String>,
    path: String,
}

impl<'a> ProcessingGuard<'a> {
    pub fn new(set: &'a DashSet<String>, path: String) -> Option<Self> {
        if set.insert(path.clone()) {
            Some(Self { set, path })
        } else {
            None
        }
    }
}

impl Drop for ProcessingGuard<'_> {
    fn drop(&mut self) {
        self.set.remove(&self.path);
    }
}
