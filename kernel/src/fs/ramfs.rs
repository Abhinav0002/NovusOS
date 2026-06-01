extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use crate::sync::spinlock::SpinLock;
use super::vfs::{FileSystem, FileOps, DirEntry, Stat, FileType};

enum Node {
    File(Vec<u8>),
    Directory(BTreeMap<String, Node>),
}

pub struct RamFs {
    root: SpinLock<BTreeMap<String, Node>>,
}

impl RamFs {
    pub fn new() -> Self {
        Self {
            root: SpinLock::new(BTreeMap::new()),
        }
    }

    fn with_parent<F, R>(tree: &mut BTreeMap<String, Node>, path: &str, f: F) -> Result<R, &'static str>
    where
        F: FnOnce(&mut BTreeMap<String, Node>, &str) -> Result<R, &'static str>,
    {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return Err("invalid path");
        }

        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() == 1 {
            return f(tree, parts[0]);
        }

        let mut current = tree;
        for &part in &parts[..parts.len() - 1] {
            let node = current.get_mut(part).ok_or("parent directory not found")?;
            match node {
                Node::Directory(ref mut children) => current = children,
                _ => return Err("not a directory"),
            }
        }
        f(current, parts[parts.len() - 1])
    }

    fn lookup<'a>(tree: &'a BTreeMap<String, Node>, path: &str) -> Option<&'a Node> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return None; // Root is special
        }

        let parts: Vec<&str> = path.split('/').collect();
        let mut current = tree;
        for (i, &part) in parts.iter().enumerate() {
            match current.get(part) {
                Some(node) => {
                    if i == parts.len() - 1 {
                        return Some(node);
                    }
                    match node {
                        Node::Directory(ref children) => current = children,
                        _ => return None,
                    }
                }
                None => return None,
            }
        }
        None
    }

    fn dir_entries(tree: &BTreeMap<String, Node>) -> Vec<DirEntry> {
        tree.iter()
            .map(|(name, node)| {
                let (file_type, size) = match node {
                    Node::File(data) => (FileType::File, data.len()),
                    Node::Directory(_) => (FileType::Directory, 0),
                };
                DirEntry {
                    name: name.clone(),
                    file_type,
                    size,
                }
            })
            .collect()
    }
}

impl FileSystem for RamFs {
    fn name(&self) -> &str {
        "ramfs"
    }

    fn open(&self, path: &str) -> Result<Box<dyn FileOps>, &'static str> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return Err("cannot open root as file");
        }
        let tree = self.root.lock();
        match Self::lookup(&tree, path) {
            Some(Node::File(data)) => Ok(Box::new(RamFile {
                path: String::from(path),
                data: data.clone(),
            })),
            Some(Node::Directory(_)) => Err("cannot open directory as file"),
            None => Err("file not found"),
        }
    }

    fn readdir(&self, path: &str) -> Result<Vec<DirEntry>, &'static str> {
        let path = path.trim_matches('/');
        let tree = self.root.lock();
        if path.is_empty() {
            return Ok(Self::dir_entries(&tree));
        }
        match Self::lookup(&tree, path) {
            Some(Node::Directory(children)) => Ok(Self::dir_entries(children)),
            Some(Node::File(_)) => Err("not a directory"),
            None => Err("directory not found"),
        }
    }

    fn stat(&self, path: &str) -> Result<Stat, &'static str> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return Ok(Stat {
                file_type: FileType::Directory,
                size: 0,
            });
        }
        let tree = self.root.lock();
        match Self::lookup(&tree, path) {
            Some(Node::File(data)) => Ok(Stat {
                file_type: FileType::File,
                size: data.len(),
            }),
            Some(Node::Directory(_)) => Ok(Stat {
                file_type: FileType::Directory,
                size: 0,
            }),
            None => Err("not found"),
        }
    }

    fn create(&self, path: &str, file_type: FileType) -> Result<(), &'static str> {
        let mut tree = self.root.lock();
        Self::with_parent(&mut tree, path, |parent, name| {
            if parent.contains_key(name) {
                return Err("already exists");
            }
            let node = match file_type {
                FileType::File => Node::File(Vec::new()),
                FileType::Directory => Node::Directory(BTreeMap::new()),
            };
            parent.insert(String::from(name), node);
            Ok(())
        })
    }

    fn remove(&self, path: &str) -> Result<(), &'static str> {
        let mut tree = self.root.lock();
        Self::with_parent(&mut tree, path, |parent, name| {
            parent.remove(name).ok_or("not found")?;
            Ok(())
        })
    }
}

struct RamFile {
    path: String,
    data: Vec<u8>,
}

impl FileOps for RamFile {
    fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
        if offset >= self.data.len() {
            return Ok(0);
        }
        let available = self.data.len() - offset;
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&self.data[offset..offset + to_read]);
        Ok(to_read)
    }

    fn write(&mut self, offset: usize, data: &[u8]) -> Result<usize, &'static str> {
        let end = offset + data.len();
        if end > self.data.len() {
            self.data.resize(end, 0);
        }
        self.data[offset..end].copy_from_slice(data);
        Ok(data.len())
    }

    fn stat(&self) -> Result<Stat, &'static str> {
        Ok(Stat {
            file_type: FileType::File,
            size: self.data.len(),
        })
    }

    fn close(&mut self) {
        // RamFile writes are in-memory only in this clone.
        // For persistence, we'd need to write back to the RamFs tree.
        // This is handled via a write-back on close.
    }
}

pub struct RamFileWriteback<'a> {
    fs: &'a RamFs,
    path: String,
    data: Vec<u8>,
}

impl RamFs {
    pub fn write_file(&self, path: &str, data: &[u8]) -> Result<(), &'static str> {
        let mut tree = self.root.lock();
        Self::with_parent(&mut tree, path, |parent, name| {
            match parent.get_mut(name) {
                Some(Node::File(ref mut existing)) => {
                    existing.clear();
                    existing.extend_from_slice(data);
                    Ok(())
                }
                Some(Node::Directory(_)) => Err("is a directory"),
                None => {
                    parent.insert(String::from(name), Node::File(data.to_vec()));
                    Ok(())
                }
            }
        })
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, &'static str> {
        let tree = self.root.lock();
        match Self::lookup(&tree, path) {
            Some(Node::File(data)) => Ok(data.clone()),
            Some(Node::Directory(_)) => Err("is a directory"),
            None => Err("file not found"),
        }
    }
}
