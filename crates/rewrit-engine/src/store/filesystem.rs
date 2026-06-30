use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RewritStore {
    pub root: PathBuf,
    pub baselines_dir: PathBuf,
    pub reports_dir: PathBuf,
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
}
