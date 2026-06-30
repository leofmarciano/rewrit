use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RewritStore {
    pub root: PathBuf,
    pub baselines_dir: PathBuf,
    pub reports_dir: PathBuf,
}

#[derive(Debug)]
pub struct StoreLock {
    path: PathBuf,
}

impl RewritStore {
    #[must_use]
    pub fn new(root: &Path, baselines_dir: Option<&str>, reports_dir: Option<&str>) -> Self {
        let rewrit_root = root.join(".rewrit");
        Self {
            root: rewrit_root.clone(),
            baselines_dir: baselines_dir
                .map(|path| root.join(path))
                .unwrap_or_else(|| rewrit_root.join("baselines")),
            reports_dir: reports_dir
                .map(|path| root.join(path))
                .unwrap_or_else(|| rewrit_root.join("reports")),
        }
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.baselines_dir)?;
        std::fs::create_dir_all(&self.reports_dir)?;
        std::fs::create_dir_all(self.root.join("cache"))?;
        std::fs::create_dir_all(self.root.join("locks"))?;
        std::fs::create_dir_all(self.root.join("tmp"))?;
        Ok(())
    }

    pub fn acquire_lock(&self, name: &str) -> std::io::Result<StoreLock> {
        let locks_dir = self.root.join("locks");
        std::fs::create_dir_all(&locks_dir)?;
        let path = locks_dir.join(format!("{}.lock", lock_name(name)));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        writeln!(file, "pid={}", std::process::id())?;
        Ok(StoreLock { path })
    }
}

impl Drop for StoreLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn lock_name(name: &str) -> String {
    let sanitized = name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => ch,
            _ => '_',
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "store".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_lock_blocks_concurrent_acquire_until_drop() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = RewritStore::new(temp.path(), None, None);
        store.ensure().expect("store");

        let lock = store.acquire_lock("reports").expect("first lock");
        let second = store
            .acquire_lock("reports")
            .expect_err("second lock should fail");
        assert_eq!(second.kind(), std::io::ErrorKind::AlreadyExists);

        drop(lock);
        let _third = store.acquire_lock("reports").expect("third lock");
    }
}
