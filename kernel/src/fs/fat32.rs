extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::virtio::block::VirtioBlock;
use crate::sync::spinlock::SpinLock;
use super::vfs::{FileSystem, FileOps, DirEntry, Stat, FileType};

struct BiosParameterBlock {
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    num_fats: u8,
    total_sectors_32: u32,
    fat_size_32: u32,
    root_cluster: u32,
}

impl BiosParameterBlock {
    fn parse(sector: &[u8; 512]) -> Option<Self> {
        if sector[510] != 0x55 || sector[511] != 0xAA {
            return None;
        }

        let bytes_per_sector = u16::from_le_bytes([sector[11], sector[12]]);
        let sectors_per_cluster = sector[13];
        let reserved_sectors = u16::from_le_bytes([sector[14], sector[15]]);
        let num_fats = sector[16];
        let total_sectors_32 = u32::from_le_bytes([sector[32], sector[33], sector[34], sector[35]]);
        let fat_size_32 = u32::from_le_bytes([sector[36], sector[37], sector[38], sector[39]]);
        let root_cluster = u32::from_le_bytes([sector[44], sector[45], sector[46], sector[47]]);

        if bytes_per_sector != 512 || sectors_per_cluster == 0 {
            return None;
        }

        Some(Self {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            num_fats,
            total_sectors_32,
            fat_size_32,
            root_cluster,
        })
    }

    fn first_data_sector(&self) -> u32 {
        self.reserved_sectors as u32 + self.num_fats as u32 * self.fat_size_32
    }

    fn cluster_to_sector(&self, cluster: u32) -> u64 {
        let first_data = self.first_data_sector();
        (first_data + (cluster - 2) * self.sectors_per_cluster as u32) as u64
    }

    fn fat_sector_for_cluster(&self, cluster: u32) -> (u64, usize) {
        let fat_offset = cluster * 4;
        let fat_sector = self.reserved_sectors as u64 + fat_offset as u64 / 512;
        let offset_in_sector = (fat_offset % 512) as usize;
        (fat_sector, offset_in_sector)
    }
}

pub struct Fat32Fs {
    block: SpinLock<VirtioBlock>,
    bpb: BiosParameterBlock,
}

impl Fat32Fs {
    pub fn new(mut block: VirtioBlock) -> Option<Self> {
        let mut sector = [0u8; 512];
        if !block.read_sector(0, &mut sector) {
            crate::println!("[fat32] Failed to read boot sector");
            return None;
        }

        let bpb = BiosParameterBlock::parse(&sector)?;

        crate::println!(
            "[fat32] Mounted: {} bytes/sector, {} sectors/cluster, root cluster {}",
            bpb.bytes_per_sector,
            bpb.sectors_per_cluster,
            bpb.root_cluster
        );

        Some(Self {
            block: SpinLock::new(block),
            bpb,
        })
    }

    fn read_cluster(&self, cluster: u32, buf: &mut Vec<u8>) -> bool {
        let start_sector = self.bpb.cluster_to_sector(cluster);
        let count = self.bpb.sectors_per_cluster as u64;
        let mut block = self.block.lock();

        for i in 0..count {
            let mut sector = [0u8; 512];
            if !block.read_sector(start_sector + i, &mut sector) {
                return false;
            }
            buf.extend_from_slice(&sector);
        }
        true
    }

    fn next_cluster(&self, cluster: u32) -> Option<u32> {
        let (fat_sector, offset) = self.bpb.fat_sector_for_cluster(cluster);
        let mut sector = [0u8; 512];
        let mut block = self.block.lock();
        if !block.read_sector(fat_sector, &mut sector) {
            return None;
        }
        let entry = u32::from_le_bytes([
            sector[offset],
            sector[offset + 1],
            sector[offset + 2],
            sector[offset + 3],
        ]) & 0x0FFF_FFFF;

        if entry >= 0x0FFF_FFF8 {
            None // End of chain
        } else {
            Some(entry)
        }
    }

    fn read_chain(&self, start_cluster: u32) -> Vec<u8> {
        let mut data = Vec::new();
        let mut cluster = start_cluster;
        loop {
            if !self.read_cluster(cluster, &mut data) {
                break;
            }
            match self.next_cluster(cluster) {
                Some(next) => cluster = next,
                None => break,
            }
        }
        data
    }

    fn parse_dir_entries(&self, data: &[u8]) -> Vec<Fat32DirEntry> {
        let mut entries = Vec::new();
        let mut lfn_parts: Vec<(u8, [u16; 13])> = Vec::new();

        for chunk in data.chunks_exact(32) {
            if chunk[0] == 0x00 {
                break; // No more entries
            }
            if chunk[0] == 0xE5 {
                lfn_parts.clear();
                continue; // Deleted
            }

            // LFN entry
            if chunk[11] == 0x0F {
                let seq = chunk[0];
                let mut name_chars = [0u16; 13];
                // chars 1-5 at offset 1
                for i in 0..5 {
                    name_chars[i] = u16::from_le_bytes([chunk[1 + i * 2], chunk[2 + i * 2]]);
                }
                // chars 6-11 at offset 14
                for i in 0..6 {
                    name_chars[5 + i] = u16::from_le_bytes([chunk[14 + i * 2], chunk[15 + i * 2]]);
                }
                // chars 12-13 at offset 28
                for i in 0..2 {
                    name_chars[11 + i] = u16::from_le_bytes([chunk[28 + i * 2], chunk[29 + i * 2]]);
                }
                lfn_parts.push((seq & 0x3F, name_chars));
                continue;
            }

            let attr = chunk[11];
            if attr & 0x08 != 0 {
                lfn_parts.clear();
                continue; // Volume label
            }

            let name = if !lfn_parts.is_empty() {
                lfn_parts.sort_by_key(|(seq, _)| *seq);
                let mut full_name = String::new();
                for (_, chars) in &lfn_parts {
                    for &c in chars {
                        if c == 0x0000 || c == 0xFFFF {
                            break;
                        }
                        if let Some(ch) = char::from_u32(c as u32) {
                            full_name.push(ch);
                        }
                    }
                }
                lfn_parts.clear();
                full_name
            } else {
                parse_8_3_name(chunk)
            };

            let file_type = if attr & 0x10 != 0 {
                FileType::Directory
            } else {
                FileType::File
            };

            let cluster_hi = u16::from_le_bytes([chunk[20], chunk[21]]) as u32;
            let cluster_lo = u16::from_le_bytes([chunk[26], chunk[27]]) as u32;
            let cluster = (cluster_hi << 16) | cluster_lo;
            let size = u32::from_le_bytes([chunk[28], chunk[29], chunk[30], chunk[31]]) as usize;

            if name == "." || name == ".." {
                continue;
            }

            entries.push(Fat32DirEntry {
                name,
                file_type,
                cluster,
                size,
            });
        }

        entries
    }

    fn find_entry(&self, path: &str) -> Option<Fat32DirEntry> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return Some(Fat32DirEntry {
                name: String::from("/"),
                file_type: FileType::Directory,
                cluster: self.bpb.root_cluster,
                size: 0,
            });
        }

        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current_cluster = self.bpb.root_cluster;

        for (i, part) in parts.iter().enumerate() {
            let dir_data = self.read_chain(current_cluster);
            let entries = self.parse_dir_entries(&dir_data);

            let found = entries.iter().find(|e| {
                e.name.eq_ignore_ascii_case(part)
            });

            match found {
                Some(entry) => {
                    if i == parts.len() - 1 {
                        return Some(entry.clone());
                    }
                    if entry.file_type != FileType::Directory {
                        return None;
                    }
                    current_cluster = entry.cluster;
                }
                None => return None,
            }
        }

        None
    }
}

#[derive(Clone)]
struct Fat32DirEntry {
    name: String,
    file_type: FileType,
    cluster: u32,
    size: usize,
}

fn parse_8_3_name(entry: &[u8]) -> String {
    let mut name = String::new();
    let base = &entry[0..8];
    let ext = &entry[8..11];

    for &b in base {
        if b == 0x20 {
            break;
        }
        name.push(if b >= b'A' && b <= b'Z' {
            (b + 32) as char
        } else {
            b as char
        });
    }

    let ext_str: String = ext
        .iter()
        .take_while(|&&b| b != 0x20)
        .map(|&b| {
            if b >= b'A' && b <= b'Z' {
                (b + 32) as char
            } else {
                b as char
            }
        })
        .collect();

    if !ext_str.is_empty() {
        name.push('.');
        name.push_str(&ext_str);
    }

    name
}

impl FileSystem for Fat32Fs {
    fn name(&self) -> &str {
        "fat32"
    }

    fn open(&self, path: &str) -> Result<Box<dyn FileOps>, &'static str> {
        let entry = self.find_entry(path).ok_or("file not found")?;
        if entry.file_type == FileType::Directory {
            return Err("cannot open directory as file");
        }
        let data = self.read_chain(entry.cluster);
        let data = data[..entry.size].to_vec();
        Ok(Box::new(Fat32File { data, size: entry.size }))
    }

    fn readdir(&self, path: &str) -> Result<Vec<DirEntry>, &'static str> {
        let entry = self.find_entry(path).ok_or("directory not found")?;
        if entry.file_type != FileType::Directory {
            return Err("not a directory");
        }
        let dir_data = self.read_chain(entry.cluster);
        let fat_entries = self.parse_dir_entries(&dir_data);
        Ok(fat_entries
            .into_iter()
            .map(|e| DirEntry {
                name: e.name,
                file_type: e.file_type,
                size: e.size,
            })
            .collect())
    }

    fn stat(&self, path: &str) -> Result<Stat, &'static str> {
        let entry = self.find_entry(path).ok_or("not found")?;
        Ok(Stat {
            file_type: entry.file_type,
            size: entry.size,
        })
    }
}

struct Fat32File {
    data: Vec<u8>,
    size: usize,
}

impl FileOps for Fat32File {
    fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
        if offset >= self.size {
            return Ok(0);
        }
        let available = self.size - offset;
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&self.data[offset..offset + to_read]);
        Ok(to_read)
    }

    fn write(&mut self, _offset: usize, _data: &[u8]) -> Result<usize, &'static str> {
        Err("fat32 is read-only")
    }

    fn stat(&self) -> Result<Stat, &'static str> {
        Ok(Stat {
            file_type: FileType::File,
            size: self.size,
        })
    }
}
