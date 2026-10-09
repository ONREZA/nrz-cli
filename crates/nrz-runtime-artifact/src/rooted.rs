//! Filesystem reads relative to an acquired artifact directory handle.
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};

/// Shared directory authority, retained across inspection, scanning, and publication.
/// Descendants remain mutable; callers must still verify the bytes they consume.
#[derive(Clone, Debug)]
pub struct ArtifactRoot(Arc<Dir>);

/// The filesystem entry kind, without exposing an ambient pathname.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactFileType {
    File,
    Directory,
    Symlink,
    Other,
}

impl ArtifactFileType {
    #[must_use]
    pub const fn is_file(self) -> bool {
        matches!(self, Self::File)
    }
    #[must_use]
    pub const fn is_dir(self) -> bool {
        matches!(self, Self::Directory)
    }
    #[must_use]
    pub const fn is_symlink(self) -> bool {
        matches!(self, Self::Symlink)
    }
}

impl ArtifactRoot {
    /// Acquire a directory owner. All subsequent paths are relative to this handle.
    pub fn open(path: &Path) -> io::Result<Self> {
        Dir::open_ambient_dir(path, ambient_authority()).map(|dir| Self(Arc::new(dir)))
    }

    /// Open a regular file, following contained ancestors but refusing a final symlink.
    pub fn open_file(&self, path: &Path) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt as _;
            options.custom_flags(libc::O_NONBLOCK);
        }
        let file = self.0.open_with(relative_path(path), &options)?.into_std();
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "artifact input must be a regular file",
            ));
        }
        Ok(file)
    }

    /// Read the same regular file handle whose type was checked by `open_file`.
    pub fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.open_file(path)?.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    /// Return exact link contents, including targets which containment will reject.
    pub fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        self.0.read_link_contents(relative_path(path))
    }

    /// Resolve symlinks before parent components, returning a root-relative path.
    pub fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        let canonical = self.0.canonicalize(relative_path(path))?;
        Ok(if canonical == Path::new(".") {
            PathBuf::new()
        } else {
            canonical
        })
    }

    /// Inspect an entry through the root owner, optionally following its final link.
    pub fn file_type(&self, path: &Path, follow: bool) -> io::Result<ArtifactFileType> {
        let path = relative_path(path);
        let kind = if follow {
            self.0.metadata(path)?
        } else {
            self.0.symlink_metadata(path)?
        }
        .file_type();
        Ok(if kind.is_file() {
            ArtifactFileType::File
        } else if kind.is_dir() {
            ArtifactFileType::Directory
        } else if kind.is_symlink() {
            ArtifactFileType::Symlink
        } else {
            ArtifactFileType::Other
        })
    }

    /// List entry names within a contained directory.
    pub fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        self.0
            .read_dir(relative_path(path))?
            .map(|entry| entry.map(|entry| PathBuf::from(entry.file_name())))
            .collect()
    }
}

fn relative_path(path: &Path) -> &Path {
    if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    }
}

#[cfg(test)]
#[path = "rooted_tests.rs"]
mod tests;
