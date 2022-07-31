use super::{
    get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType, EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};

pub struct Inode {
    inode_id: usize,
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
    links: Mutex<Vec<usize>>,
}

impl Inode {
    pub fn new(
        inode_id: u32,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
        links: Vec<usize>,
    ) -> Self {
        let (block_id, block_offset) = fs.lock().get_disk_inode_pos(inode_id);
        Self {
            inode_id: inode_id as usize,
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
            links: Mutex::new(links),
        }
    }

    pub fn get_inode_id(&self) -> usize {
        self.inode_id
    }

    pub fn get_inode_dev(&self) -> usize {
        self.block_id
    }

    pub fn get_inode_links(&self, id: usize) -> usize {
        self.links.lock()[id]
    }

    pub fn is_dir(&self) -> bool {
        self.modify_disk_inode(|inode| inode.is_dir())
    }

    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }

    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }

    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_number() as u32);
            }
        }
        None
    }

    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let _ = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                Arc::new(Self::new(
                    inode_id,
                    self.fs.clone(),
                    self.block_device.clone(),
                    Vec::new(),
                ))
            })
        })
    }

    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    pub fn linkat(&self, newname: &str, oldname: &str) -> isize {
        if self.find(newname).is_some() {
            log::trace!("{} exist", newname);
            return -1;
        }
        if self.find(oldname).is_none() {
            log::trace!("{} not exist", oldname);
            return -1;
        }
        if let Some(old_inode) = self.find(oldname) {
            self.modify_disk_inode(|root_inode| {
                // append file in the dirent
                let file_count = (root_inode.size as usize) / DIRENT_SZ;
                let new_size = (file_count + 1) * DIRENT_SZ;
                let link_id = old_inode.get_inode_id();
                self.links.lock()[link_id] += 1;
                // increase size
                let mut fs = self.fs.lock();
                self.increase_size(new_size as u32, root_inode, &mut fs);
                drop(fs);
                // write dirent
                let dirent = DirEntry::new(newname, link_id as u32);
                root_inode.write_at(
                    file_count * DIRENT_SZ,
                    dirent.as_bytes(),
                    &self.block_device,
                );
            });
            0
        } else {
            -1
        }
    }
    pub fn unlinkat(&self, path: &str) -> isize {
        if let Some(inode) = self.find(path) {
            let remove_id = inode.get_inode_id();
            let new_inode_id = self.fs.lock().alloc_inode();
            self.modify_disk_inode(|root_inode| {
                // append file in the dirent
                let mut dirent = DirEntry::empty();
                let file_count = (root_inode.size as usize) / DIRENT_SZ;
                for i in 0..file_count {
                    assert_eq!(
                        root_inode.read_at(
                            DIRENT_SZ * i,
                            dirent.as_bytes_mut(),
                            &self.block_device,
                        ),
                        DIRENT_SZ,
                    );
                    if dirent.name() == path {
                        self.links.lock()[remove_id] -= 1;
                        let dirent = DirEntry::new(path, new_inode_id);
                        root_inode.write_at(i * DIRENT_SZ, dirent.as_bytes(), &self.block_device);
                        break;
                    }
                }
                return 0;
            });
        }
        -1
    }

    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        if self
            .modify_disk_inode(|root_inode| {
                // assert it is a directory
                assert!(root_inode.is_dir());
                // has the file been created?
                self.find_inode_id(name, root_inode)
            })
            .is_some()
        {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });
        // release efs lock manually because we will acquire it again in Inode::new
        drop(fs);
        // return inode
        Some(Arc::new(Self::new(
            new_inode_id,
            self.fs.clone(),
            self.block_device.clone(),
            Vec::new(),
        )))
    }

    pub fn ls(&self) -> Vec<String> {
        let _ = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _ = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }

    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        })
    }

    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
    }
}
