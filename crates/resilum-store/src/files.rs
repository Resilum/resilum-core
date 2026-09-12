use std::io;
use std::path::{Path, PathBuf};

pub fn read_text(path: &Path) -> io::Result<String> {
    std::fs::read_to_string(path)
}

pub fn read_bytes(path: &Path) -> io::Result<Vec<u8>> {
    std::fs::read(path)
}

pub fn write_text(path: &Path, text: &str) -> io::Result<()> {
    std::fs::write(path, text)
}

pub fn write_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    std::fs::write(path, bytes)
}

#[cfg(unix)]
pub fn own_eyes_only(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
pub fn own_eyes_only(_path: &Path) -> io::Result<()> {
    Ok(())
}

pub fn make_room_for(files: &Path) -> io::Result<()> {
    std::fs::create_dir_all(files)
}

pub fn forget(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

pub fn size_of(path: &Path) -> io::Result<u64> {
    Ok(std::fs::metadata(path)?.len())
}

pub fn modified_at(path: &Path) -> io::Result<std::time::SystemTime> {
    std::fs::metadata(path)?.modified()
}

pub fn list(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut listed = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        listed.push(entry?.path());
    }
    Ok(listed)
}

pub fn replace_with(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        make_room_for(parent)?;
    }
    let beside = path.with_extension("tmp");
    write_and_sync(&beside, bytes)?;
    std::fs::rename(&beside, path)?;
    if let Some(parent) = path.parent() {
        sync_the_directory_so_the_rename_outlives_a_power_cut(parent);
    }
    Ok(())
}

fn write_and_sync(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn sync_the_directory_so_the_rename_outlives_a_power_cut(parent: &Path) {
    match std::fs::File::open(parent).and_then(|dir| dir.sync_all()) {
        Ok(()) => {}
        Err(e) => tracing::debug!(error = %e, "a store's directory was not flushed"),
    }
}
