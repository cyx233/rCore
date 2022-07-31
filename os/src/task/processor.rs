use super::TaskControlBlock;
use super::__switch;
use super::{fetch_task, TaskStatus};
use crate::trap::TrapContext;
use alloc::sync::Arc;
use core::cell::RefCell;
use lazy_static::*;

use crate::mm::{MapPermission, VirtAddr};

pub struct Processor {
    inner: RefCell<ProcessorInner>,
}

unsafe impl Sync for Processor {}

struct ProcessorInner {
    current: Option<Arc<TaskControlBlock>>,
    idle_task_cx_ptr: usize,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(ProcessorInner {
                current: None,
                idle_task_cx_ptr: 0,
            }),
        }
    }
    fn get_idle_task_cx_ptr2(&self) -> *const usize {
        let inner = self.inner.borrow();
        &inner.idle_task_cx_ptr as *const usize
    }
    pub fn run(&self) {
        loop {
            if let Some(task) = fetch_task() {
                let idle_task_cx_ptr2 = self.get_idle_task_cx_ptr2();
                // acquire
                let mut task_inner = task.acquire_inner_lock();
                let next_task_cx_ptr2 = task_inner.get_task_cx_ptr2();
                task_inner.task_status = TaskStatus::Running;
                drop(task_inner);
                // release
                self.inner.borrow_mut().current = Some(task);
                unsafe {
                    __switch(idle_task_cx_ptr2, next_task_cx_ptr2);
                }
            }
        }
    }
    pub fn take_current(&self) -> Option<Arc<TaskControlBlock>> {
        self.inner.borrow_mut().current.take()
    }
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.inner
            .borrow()
            .current
            .as_ref()
            .map(|task| Arc::clone(task))
    }
    pub fn set_priority(&self, prio: isize) {
        let task = current_task().unwrap();
        // ---- hold current PCB lock
        let mut task_inner = task.acquire_inner_lock();
        task_inner.task_priority = prio as usize;
        drop(task_inner);
    }
    pub fn map_dynamic(
        &self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_perm: MapPermission,
    ) -> isize {
        let task = current_task().unwrap();
        // ---- hold current PCB lock
        let mut task_inner = task.acquire_inner_lock();
        task_inner
            .memory_set
            .map_dynamic(start_va, end_va, map_perm)
    }
    pub fn unmap_dynamic(&self, start_va: VirtAddr, end_va: VirtAddr) -> isize {
        let task = current_task().unwrap();
        // ---- hold current PCB lock
        let mut task_inner = task.acquire_inner_lock();
        task_inner.memory_set.unmap_dynamic(start_va, end_va)
    }
}

lazy_static! {
    pub static ref PROCESSOR: Processor = Processor::new();
}

pub fn run_tasks() {
    PROCESSOR.run();
}

pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.take_current()
}

pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.current()
}

pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.acquire_inner_lock().get_user_token();
    token
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task().unwrap().acquire_inner_lock().get_trap_cx()
}

pub fn schedule(switched_task_cx_ptr2: *const usize) {
    let idle_task_cx_ptr2 = PROCESSOR.get_idle_task_cx_ptr2();
    unsafe {
        __switch(switched_task_cx_ptr2, idle_task_cx_ptr2);
    }
}
