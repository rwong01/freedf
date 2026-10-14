//! All document writes use private sibling temporary files and atomic replacement.
use anyhow::{Context, Result, ensure};
use std::{fs, io::Write, path::Path};

pub fn read_pdf(path: &Path) -> Result<Vec<u8>> {
    use std::io::Read;
    let f = fs::File::open(path).context("Cannot open this file")?;
    ensure!(
        f.metadata()?.len() <= 256 * 1024 * 1024,
        "PDF exceeds the 256 MiB safety limit"
    );
    let mut out = vec![];
    f.take(256 * 1024 * 1024 + 1).read_to_end(&mut out)?;
    ensure!(
        out.len() <= 256 * 1024 * 1024,
        "PDF grew beyond the safety limit while reading"
    );
    Ok(out)
}

pub fn save_pdf(path: &Path, bytes: &[u8], expected: Option<&[u8]>) -> Result<()> {
    let parsed =
        lopdf::Document::load_mem(bytes).context("Save aborted: output is not a readable PDF")?;
    ensure!(
        !parsed.get_pages().is_empty(),
        "Save aborted: output has no pages"
    );
    if let Some(expected) = expected {
        let current = read_pdf(path)
            .context("The original file disappeared or cannot be checked. Use Save As.")?;
        ensure!(
            current == expected,
            "The file changed in another application. Use Save As to preserve both versions."
        );
    }
    atomic_write(path, bytes)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut tmp = tempfile::Builder::new()
        .prefix(".openpdf-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .context("Cannot create a temporary file beside the destination")?;
    tmp.write_all(bytes)
        .context("Could not write the complete PDF; the destination was not replaced")?;
    tmp.as_file()
        .sync_all()
        .context("Could not synchronize the PDF; the destination was not replaced")?;
    tmp.persist(path).map_err(|e|e.error).context("Could not replace the destination; check permissions or close applications locking the file")?;
    #[cfg(unix)]
    fs::File::open(parent)?
        .sync_all()
        .context("PDF replaced, but the directory could not be synchronized")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failures_leave_original_and_clean_temporaries() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.pdf");
        fs::write(&p, b"original").unwrap();
        assert!(save_pdf(&p, b"broken", None).is_err());
        assert_eq!(fs::read(&p).unwrap(), b"original");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        assert!(atomic_write(&dir.path().join("missing/a.pdf"), b"x").is_err());
    }
    #[test]
    fn replace_existing() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a");
        atomic_write(&p, b"one").unwrap();
        atomic_write(&p, b"two").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"two");
    }
}
