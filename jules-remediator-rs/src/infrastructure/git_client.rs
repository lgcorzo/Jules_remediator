use anyhow::{Result, Context};
use std::process::Command;
use std::path::PathBuf;

pub struct GitClient {
    pub repo_path: PathBuf,
}

impl GitClient {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    pub fn clone_repo(&self, url: &str) -> Result<()> {
        if self.repo_path.exists() {
            println!("[GitClient] Repo already exists at {:?}", self.repo_path);
            return Ok(());
        }

        let output = Command::new("git")
            .arg("clone")
            .arg(url)
            .arg(&self.repo_path)
            .output()
            .context("failed to clone repo")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("failed to clone repo: {}", err);
        }

        Ok(())
    }

    pub fn create_branch(&self, branch_name: &str) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .arg("checkout")
            .arg("-b")
            .arg(branch_name)
            .output()
            .context("failed to create branch")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("failed to create branch: {}", err);
        }

        Ok(())
    }

    pub fn commit_all(&self, message: &str) -> Result<()> {
        // Add all changes
        let add_output = Command::new("git")
            .current_dir(&self.repo_path)
            .arg("add")
            .arg(".")
            .output()
            .context("failed to git add")?;

        if !add_output.status.success() {
            let err = String::from_utf8_lossy(&add_output.stderr);
            anyhow::bail!("git add failed: {}", err);
        }

        // Commit
        let commit_output = Command::new("git")
            .current_dir(&self.repo_path)
            .arg("commit")
            .arg("-m")
            .arg(message)
            .output()
            .context("failed to git commit")?;

        if !commit_output.status.success() {
            let err = String::from_utf8_lossy(&commit_output.stderr);
            anyhow::bail!("git commit failed: {}", err);
        }

        Ok(())
    }

    pub fn push(&self, branch_name: &str) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .arg("push")
            .arg("origin")
            .arg(branch_name)
            .output()
            .context("failed to git push")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git push failed: {}", err);
        }

        Ok(())
    }
}
