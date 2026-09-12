mod document;
mod files;
mod writer;

pub use document::Document;
pub use files::{
    forget, list, make_room_for, modified_at, own_eyes_only, read_bytes, read_text, replace_with,
    size_of, write_bytes, write_text,
};
pub use writer::Writer;
