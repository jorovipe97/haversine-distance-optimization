use anyhow::{Context, Result};
use std::fs::{self, File, remove_file};
use std::io::Write;
use std::path::PathBuf;

pub struct TemporaryTestFile {
    file_path: PathBuf,
}

impl TemporaryTestFile {
    /// File name must have extension.
    pub fn new(file_name: &str, file_content: &str) -> Result<TemporaryTestFile> {
        let mut file_path = std::env::temp_dir();
        file_path.push("temporary-test-files");
        file_path.push(&file_name);

        println!("will create temp file in: {:?}", &file_path);

        // Creates all parent files that are missing
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).context("could not create parent directories")?;
        }

        let mut file = File::create(&file_path).context("could not create file")?;
        file.write_all(file_content.as_bytes())
            .context("could not write content to temporary file")?;

        Ok(TemporaryTestFile { file_path })
    }

    pub fn full_file_path(&self) -> Option<&str> {
        self.file_path.to_str()
    }

    pub fn cleanup(&self) -> Result<()> {
        remove_file(&self.file_path).context("could not remove temporary file")?;

        println!("removed temp file at {:?}", &self.file_path);
        Ok(())
    }
}
