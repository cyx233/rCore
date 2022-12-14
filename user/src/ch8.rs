pub use super::*;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use rand::distributions::{Distribution, Standard};
use rand::rngs::SmallRng;
use rand::{Fill, Rng, SeedableRng};
use spin::mutex::Mutex;

pub const PAGE_SIZE: usize = 4096;
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;
const SYSCALL_OPENAT: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_UNLINKAT: usize = 35;
const SYSCALL_LINKAT: usize = 37;
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GETTIMEOFDAY: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_SET_PRIORITY: usize = 140;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_SPAWN: usize = 400;
const SYSCALL_MAIL_READ: usize = 401;
const SYSCALL_MAIL_WRITE: usize = 402;

pub fn forktest<F>(func: F)
where
    F: FnOnce(usize),
{
    let n: usize = 200;
    let mut cnt = 0; // 统计 fork 成功的次数
    for idx in 0..n {
        let pid = fork();
        if pid == 0 {
            func(idx);
            exit(0);
        } else if pid > 0 {
            cnt += 1;
        }
    }
    let mut exit_code: i32 = 0;
    for _ in 0..cnt {
        assert!(wait(&mut exit_code) > 0);
        assert_eq!(exit_code, 0);
    }
    assert!(wait(&mut exit_code) < 0);
}
pub fn get_pc() -> usize {
    let mut ra: usize;
    unsafe {
        llvm_asm!("mv $0, ra" : "=r"(ra) ::: "volatile");
    }
    ra
}

pub fn raw_sys_gettime(tx: *const TimeVal, tz: usize) -> isize {
    syscall(SYSCALL_GETTIMEOFDAY, [tx as usize, tz, 0])
}

pub fn raw_sys_fstat(fd: usize, st: *const Stat) -> isize {
    syscall(SYSCALL_FSTAT, [fd, st as usize, 0])
}

pub fn raw_syscall(id: usize, args: [usize; 5]) -> isize {
    syscall5(id, args)
}

lazy_static! {
    static ref PRNG: Arc<Mutex<SmallRng>> = {
        type Seed = [u8; 32];
        Arc::new(Mutex::new(SmallRng::from_seed(Seed::default())))
    };
}

pub fn xorshift64(mut x: usize) -> usize {
    x = x ^ x << 13;
    x = x ^ x >> 7;
    x = x ^ x << 17;
    x
}

pub fn rand<T>() -> T
where
    Standard: Distribution<T>,
{
    let mut rng = PRNG.lock();
    rng.gen()
}

pub fn fill<T: Fill + ?Sized>(dest: &mut T) {
    let mut rng = PRNG.lock();
    rng.fill(dest)
}

pub fn hash(x: usize) -> usize {
    xorshift64(x)
}

fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        llvm_asm!("ecall"
            : "={x10}" (ret)
            : "{x10}" (args[0]), "{x11}" (args[1]), "{x12}" (args[2]), "{x17}" (id)
            : "memory"
            : "volatile"
        );
    }
    ret
}

fn syscall5(id: usize, args: [usize; 5]) -> isize {
    let mut ret: isize;
    unsafe {
        llvm_asm!("ecall"
            : "={x10}" (ret)
            : "{x10}" (args[0]), "{x11}" (args[1]), "{x12}" (args[2]), "{x13}" (args[3]),
                "{x14}" (args[4]), "{x17}" (id)
            : "memory"
            : "volatile"
        );
    }
    ret
}
