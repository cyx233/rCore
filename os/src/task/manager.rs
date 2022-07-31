use super::TaskControlBlock;
use crate::mm::UserBuffer;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
use spin::Mutex;

pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
    pub fn send_mail_to_pid(&mut self, pid: usize, buf: UserBuffer) -> isize {
        for i in 0..self.ready_queue.len() {
            if self.ready_queue[i].getpid() == pid {
                if let Some(file) = &self.ready_queue[i].acquire_inner_lock().fd_table[3] {
                    let file = file.clone();
                    return file.write(buf) as isize;
                } else {
                    return -1;
                }
            }
        }
        return -1;
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.lock().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.lock().fetch()
}
