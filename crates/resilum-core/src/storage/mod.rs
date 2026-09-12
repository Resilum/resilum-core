mod writer;

pub use writer::Writer;

use std::path::Path;

pub fn make_room_for(files: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(files)
}
