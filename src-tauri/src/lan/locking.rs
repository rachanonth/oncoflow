use std::{
    fs::{self, File, OpenOptions},
    io,
    path::Path,
};

pub(crate) fn lock_database(directory: &Path) -> io::Result<File> {
    fs::create_dir_all(directory)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("oncoflow-server.lock"))?;
    file.try_lock().map_err(|_| {
        io::Error::other("Another OncoFlow process is already using this data directory")
    })?;
    Ok(file)
}
