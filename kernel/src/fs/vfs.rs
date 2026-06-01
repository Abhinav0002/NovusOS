extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::sync::spinlock::SpinLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub file_type: FileType,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct Stat {
    pub file_type: FileType,
    pub size: usize,
}

pub trait FileOps: Send {
    fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str>;
    fn write(&mut self, offset: usize, data: &[u8]) -> Result<usize, &'static str>;
    fn stat(&self) -> Result<Stat, &'static str>;
    fn close(&mut self) {}
}

pub trait FileSystem: Send {
    fn name(&self) -> &str;
    fn open(&self, path: &str) -> Result<Box<dyn FileOps>, &'static str>;
    fn readdir(&self, path: &str) -> Result<Vec<DirEntry>, &'static str>;
    fn stat(&self, path: &str) -> Result<Stat, &'static str>;
    fn create(&self, path: &str, file_type: FileType) -> Result<(), &'static str> {
        let _ = (path, file_type);
        Err("read-only filesystem")
    }
    fn remove(&self, path: &str) -> Result<(), &'static str> {
        let _ = path;
        Err("read-only filesystem")
    }
}

struct MountPoint {
    path: String,
    fs: Box<dyn FileSystem>,
}

pub struct VFS {
    mounts: Vec<MountPoint>,
}

static VFS_INSTANCE: SpinLock<Option<VFS>> = SpinLock::new(None);

impl VFS {
    fn new() -> Self {
        Self {
            mounts: Vec::new(),
        }
    }

    fn mount_inner(&mut self, path: &str, fs: Box<dyn FileSystem>) {
        let path = normalize_mount_path(path);
        crate::println!("[vfs] Mounting '{}' at {}", fs.name(), path);
        if let Some(existing) = self.mounts.iter().position(|m| m.path == path) {
            self.mounts.remove(existing);
        }
        self.mounts.push(MountPoint {
            path,
            fs,
        });
        self.mounts.sort_by(|a, b| b.path.len().cmp(&a.path.len()));
    }

    fn resolve<'a>(&'a self, path: &'a str) -> Option<(&'a dyn FileSystem, &'a str)> {
        let path = if path.is_empty() { "/" } else { path };
        for mount in &self.mounts {
            if mount.path == "/" {
                let relative = path.strip_prefix('/').unwrap_or(path);
                return Some((mount.fs.as_ref(), relative));
            }
            if path == mount.path || path.starts_with(&format!("{}/", mount.path)) {
                let relative = &path[mount.path.len()..];
                let relative = relative.strip_prefix('/').unwrap_or(relative);
                return Some((mount.fs.as_ref(), relative));
            }
        }
        None
    }
}

fn normalize_mount_path(path: &str) -> String {
    if path == "/" {
        return String::from("/");
    }
    let p = path.trim_end_matches('/');
    if p.is_empty() {
        String::from("/")
    } else {
        String::from(p)
    }
}

pub fn init() {
    let mut vfs = VFS_INSTANCE.lock();
    *vfs = Some(VFS::new());
}

pub fn mount(path: &str, fs: Box<dyn FileSystem>) {
    let mut vfs = VFS_INSTANCE.lock();
    if let Some(ref mut v) = *vfs {
        v.mount_inner(path, fs);
    }
}

pub fn open(path: &str) -> Result<Box<dyn FileOps>, &'static str> {
    let vfs = VFS_INSTANCE.lock();
    let v = vfs.as_ref().ok_or("VFS not initialized")?;
    let (fs, relative) = v.resolve(path).ok_or("no filesystem mounted for path")?;
    fs.open(relative)
}

pub fn readdir(path: &str) -> Result<Vec<DirEntry>, &'static str> {
    let vfs = VFS_INSTANCE.lock();
    let v = vfs.as_ref().ok_or("VFS not initialized")?;
    let (fs, relative) = v.resolve(path).ok_or("no filesystem mounted for path")?;
    fs.readdir(relative)
}

pub fn stat(path: &str) -> Result<Stat, &'static str> {
    let vfs = VFS_INSTANCE.lock();
    let v = vfs.as_ref().ok_or("VFS not initialized")?;
    let (fs, relative) = v.resolve(path).ok_or("no filesystem mounted for path")?;
    fs.stat(relative)
}

pub fn create(path: &str, file_type: FileType) -> Result<(), &'static str> {
    let vfs = VFS_INSTANCE.lock();
    let v = vfs.as_ref().ok_or("VFS not initialized")?;
    let (fs, relative) = v.resolve(path).ok_or("no filesystem mounted for path")?;
    fs.create(relative, file_type)
}

pub fn remove(path: &str) -> Result<(), &'static str> {
    let vfs = VFS_INSTANCE.lock();
    let v = vfs.as_ref().ok_or("VFS not initialized")?;
    let (fs, relative) = v.resolve(path).ok_or("no filesystem mounted for path")?;
    fs.remove(relative)
}
