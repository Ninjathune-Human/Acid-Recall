//! The iPad build's stand-in for the few `rfd` calls (docs/ipad.md): rfd has no iOS backend.
//! The file pickers return nothing, so their buttons do nothing yet, and the crash box is not
//! shown (the crash log is still written). The builders take the same arguments as rfd's, so
//! the call sites stay as they are, with `use crate::ios_rfd as rfd;` on iOS.

use std::path::{Path, PathBuf};

pub struct FileDialog;

impl FileDialog {
    pub fn new() -> Self {
        Self
    }

    pub fn set_title(self, _title: impl Into<String>) -> Self {
        self
    }

    pub fn add_filter(self, _name: impl Into<String>, _extensions: &[impl ToString]) -> Self {
        self
    }

    pub fn set_directory<P: AsRef<Path>>(self, _path: P) -> Self {
        self
    }

    pub fn set_file_name(self, _name: impl Into<String>) -> Self {
        self
    }

    pub fn pick_file(self) -> Option<PathBuf> {
        None
    }

    pub fn save_file(self) -> Option<PathBuf> {
        None
    }
}

pub enum MessageLevel {
    Error,
}

pub enum MessageButtons {
    Ok,
}

pub struct MessageDialog;

impl MessageDialog {
    pub fn new() -> Self {
        Self
    }

    pub fn set_level(self, _level: MessageLevel) -> Self {
        self
    }

    pub fn set_title(self, _title: impl Into<String>) -> Self {
        self
    }

    pub fn set_description(self, _text: impl Into<String>) -> Self {
        self
    }

    pub fn set_buttons(self, _buttons: MessageButtons) -> Self {
        self
    }

    pub fn show(self) {}
}
