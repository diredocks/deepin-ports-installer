use std::fs;
use std::path::PathBuf;

pub(crate) struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub(crate) fn new(prefix: &str, name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "deepin-ports-sidecar-{prefix}-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create test directory");
        Self { path }
    }

    pub(crate) fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
