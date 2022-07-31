mod context;
mod manager;
mod pid;
mod processor;
mod switch;
mod task;

use crate::fs::{open_file, OpenFlags};
use crate::mm::{MapPermission, UserBuffer, VirtAddr};
use crate::task::processor::PROCESSOR;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

use alloc::sync::Arc;
pub use context::TaskContext;
use lazy_static::*;
use manager::{fetch_task, TASK_MANAGER};

pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
};

pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- hold current PCB lock
    let mut task_inner = task.acquire_inner_lock();
    let task_cx_ptr2 = task_inner.get_task_cx_ptr2();
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB lock

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr2);
}

pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();
    // **** hold current PCB lock
    let mut inner = task.acquire_inner_lock();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ hold initproc PCB lock here
    {
        let mut initproc_inner = INITPROC.acquire_inner_lock();
        for child in inner.children.iter() {
            child.acquire_inner_lock().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB lock here

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** release current PCB lock
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let _unused: usize = 0;
    schedule(&_unused as *const _);
}

lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        let inode = open_file("ch5_usershell", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    });
}

pub fn add_initproc() {
    manager::add_task(INITPROC.clone());
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    manager::add_task(task);
}

pub fn set_priority(_prio: isize) {
    PROCESSOR.set_priority(_prio);
}

pub fn map_dynamic(start_va: VirtAddr, end_va: VirtAddr, map_perm: MapPermission) -> isize {
    PROCESSOR.map_dynamic(start_va, end_va, map_perm)
}
pub fn unmap_dynamic(start_va: VirtAddr, end_va: VirtAddr) -> isize {
    PROCESSOR.unmap_dynamic(start_va, end_va)
}
pub fn send_mail_to_pid(pid: usize, buf: UserBuffer) -> isize {
    TASK_MANAGER.lock().send_mail_to_pid(pid, buf)
}
