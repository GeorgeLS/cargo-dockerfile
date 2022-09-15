use crate::{LIB_RS, MAIN_RS};
use walkdir::DirEntry;

pub fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

pub fn is_entry_of_interest(entry: &DirEntry) -> bool {
    [MAIN_RS, LIB_RS]
        .iter()
        .any(|f| entry.file_name().to_str().map(|s| s == *f).unwrap_or(false))
}

pub fn entry_predicate(entry: &DirEntry) -> bool {
    entry.path().is_file() && is_entry_of_interest(entry)
}
