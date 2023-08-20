use anyhow::Context;
use std::io::{Read, Write};
use std::path::Path;
use walkdir::WalkDir;

pub fn archive_directory(dir: &Path, out_file: &mut std::fs::File) -> anyhow::Result<()> {
    let options = zip::write::FileOptions::default().unix_permissions(0o755);
    let mut zip = zip::ZipWriter::new(out_file);
    let mut buffer = Vec::new();
    for entry in WalkDir::new(&dir).into_iter() {
        let entry = entry.context("Serializing files")?;
        let path = entry.path();
        let name = path
            .strip_prefix(&dir)?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Bad file name"))?;

        if path.is_file() {
            zip.start_file(name, options)?;
            let mut f = std::fs::File::open(path)?;
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear();
        } else if !name.is_empty() {
            zip.add_directory(name, options)?;
        }
    }
    Ok(())
}
