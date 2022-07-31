mod inode;
mod mail;
mod pipe;
mod stdio;

use crate::mm::UserBuffer;

pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
    fn get_stat(&self) -> Stat {
        Stat {
            dev: 0,
            ino: 0,
            mode: StatMode::from_bits_truncate(0),
            nlink: 1,
            pad: [0; 7],
        }
    }
}

pub use inode::{linkat, list_apps, open_file, unlinkat, OSInode, OpenFlags};
pub use mail::MailBox;
pub use pipe::{make_pipe, Pipe};
pub use stdio::{Stdin, Stdout};

#[repr(C)]
#[derive(Debug)]
pub struct Stat {
    /// 文件所在磁盘驱动器号
    pub dev: u64,
    /// inode 文件所在 inode 编号
    pub ino: u64,
    /// 文件类型
    pub mode: StatMode,
    /// 硬链接数量，初始为1
    pub nlink: u32,
    /// 无需考虑，为了兼容性设计
    pad: [u64; 7],
}
bitflags! {
    pub struct StatMode: u32 {
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;
        /// ordinary regular file
        const FILE  = 0o100000;
    }
}
