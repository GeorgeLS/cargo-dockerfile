use crate::{LIB_RS, MAIN_RS};
use walkdir::DirEntry;

pub fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|s| s.starts_with('.'))
}

pub fn is_entry_of_interest(entry: &DirEntry) -> bool {
    [MAIN_RS, LIB_RS]
        .iter()
        .any(|f| entry.file_name().to_str().is_some_and(|s| s == *f))
}

pub fn entry_predicate(entry: &DirEntry) -> bool {
    entry.path().is_file() && is_entry_of_interest(entry)
}
