use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
};

use crate::task::{Task, TaskId};

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<Mutex<VecDeque<TaskId>>>,
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<Mutex<VecDeque<TaskId>>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        self.task_queue.lock().unwrap().push_back(self.task_id);
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<Mutex<VecDeque<TaskId>>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already exists");
        }
        self.task_queue.lock().unwrap().push_back(task_id);
    }

    fn run_ready_tasks(&mut self) {
        while let Some(task_id) = self.task_queue.lock().unwrap().pop_front() {
            let task = match self.tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue, // task no longer exists (probably sporadic wake)
            };
            let waker = self
                .waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, self.task_queue.clone()));
            let mut context = Context::from_waker(waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    self.tasks.remove(&task_id);
                    self.waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    pub fn run_until_complete(&mut self) {
        while !self.tasks.is_empty() {
            self.run_ready_tasks();
        }
    }
}
