use core::mem::size_of;

use crate::config::PAGE_SIZE;
use crate::fs::{open_file, OpenFlags};
use crate::mm::{
    translated_byte_buffer, translated_ref, translated_refmut, translated_str, MapPermission,
    VirtAddr,
};
use crate::task::{
    add_task, current_task, current_user_token, exit_current_and_run_next, map_dynamic,
    set_priority, suspend_current_and_run_next, unmap_dynamic,
};
use crate::timer::get_time_us;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    log::info!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time(time: usize, _tz: usize) -> isize {
    let satp = current_user_token();
    let mut time_val_vec = translated_byte_buffer(satp, time as *const u8, size_of::<TimeVal>());
    unsafe {
        let mut time_val = time_val_vec[0].as_mut_ptr() as *mut TimeVal;
        (*time_val).sec = get_time_us() / 1000000;
        (*time_val).usec = get_time_us() % 1000000;
    }
    0
}

pub fn sys_set_priority(prio: isize) -> isize {
    if prio < 2 {
        return -1;
    }
    set_priority(prio);
    prio
}

pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    if (prot & !0x7 != 0) || (prot & 0x7 == 0) || start % PAGE_SIZE != 0 {
        return -1;
    }
    let map_perm = MapPermission::from_bits((prot << 1) as u8).unwrap() | MapPermission::U;

    map_dynamic(VirtAddr(start), VirtAddr(start + len), map_perm)
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    unmap_dynamic(VirtAddr(start), VirtAddr(start + len))
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    if let Some(new_task) = current_task.fork() {
        let new_pid = new_task.pid.0;
        // modify trap context of new_task, because it returns immediately after switching
        let trap_cx = new_task.acquire_inner_lock().get_trap_cx();
        // we do not have to move to next instruction since we have done it before
        // for child process, fork returns 0
        trap_cx.x[10] = 0;
        // add new task to scheduler
        add_task(new_task);
        log::debug!("fork pid={}", new_pid);
        new_pid as isize
    } else {
        log::info!("Fork Failed");
        return -1;
    }
}

pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    let mut args_vec: Vec<String> = Vec::new();
    loop {
        let arg_str_ptr = *translated_ref(token, args);
        if arg_str_ptr == 0 {
            break;
        }
        args_vec.push(translated_str(token, arg_str_ptr as *const u8));
        unsafe {
            args = args.add(1);
        }
    }
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        let argc = args_vec.len();
        task.exec(all_data.as_slice(), args_vec);
        // return argc because cx.x[10] will be covered with it later
        argc as isize
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();
    // find a child process

    // ---- hold current PCB lock
    let mut inner = task.acquire_inner_lock();
    if inner
        .children
        .iter()
        .find(|p| pid == -1 || pid as usize == p.getpid())
        .is_none()
    {
        return -1;
        // ---- release current PCB lock
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily hold child PCB lock
        p.acquire_inner_lock().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB lock
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily hold child lock
        let exit_code = child.acquire_inner_lock().exit_code;
        // ++++ release child PCB lock
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        log::trace!("exit code:{}", exit_code);
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB lock automatically
}

pub fn sys_spawn(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    let args_vec: Vec<String> = Vec::new();

    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        if let Some(new_task) = current_task().unwrap().fork() {
            let new_pid = new_task.pid.0;
            let trap_cx = new_task.acquire_inner_lock().get_trap_cx();
            trap_cx.x[10] = 0;
            let all_data = app_inode.read_all();
            new_task.exec(all_data.as_slice(), args_vec);
            add_task(new_task);
            new_pid as isize
        } else {
            log::info!("Spawn Failed!");
            return -1;
        }
    } else {
        -1
    }
}
