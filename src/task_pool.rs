use futures::future::BoxFuture;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct TaskError {
    message: String,
}
impl TaskError {
    pub fn new(msg: String) -> TaskError {
        TaskError { message: msg }
    }
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub type TaskRequest = Box<dyn FnOnce() -> BoxFuture<'static, ()> + Send + Sync + 'static>;

#[derive(Default)]
pub struct TaskManagerData {
    allowed_at_once: usize,
    futures: VecDeque<(u128, TaskRequest)>,
    running: HashMap<u128, JoinHandle<()>>,
}

impl TaskManagerData {
    pub fn new(allowed_at_once: usize) -> TaskManagerData {
        TaskManagerData {
            futures: VecDeque::new(),
            running: HashMap::new(),
            allowed_at_once,
        }
    }

    pub fn len(&self) -> usize {
        self.futures.len()
    }

    pub fn queue(&mut self, f: TaskRequest) -> u128 {
        let task = f;

        let id = uuid::Uuid::new_v4().as_u128();
        self.futures.push_back((id, task));

        // return id of the task to check on it later
        id
    }

    pub fn is_running(&self, id: u128) -> bool {
        self.running.contains_key(&id)
    }

    pub fn is_empty(&self) -> bool {
        self.futures.is_empty() && self.running.is_empty()
    }

    pub fn maximum_at_once(&self) -> usize {
        self.allowed_at_once
    }
}

#[derive(Clone)]
pub struct TaskPool {
    data: Arc<RwLock<TaskManagerData>>,
}

impl TaskPool {
    pub fn new(max_allowed: usize) -> TaskPool {
        TaskPool {
            data: Arc::new(RwLock::new(TaskManagerData::new(max_allowed))),
        }
    }

    /// Gets how many task are currently in our manager queue
    pub async fn len(&self) -> usize {
        let data = self.data.read().await;
        data.len()
    }

    pub async fn queue(&self, f: TaskRequest) -> u128 {
        let mut data = self.data.write().await;
        data.queue(f)
    }

    pub async fn is_running(&self, id: u128) -> bool {
        let data = self.data.read().await;
        data.is_running(id)
    }

    pub async fn is_empty(&self) -> bool {
        let data = self.data.read().await;
        data.is_empty()
    }

    /// steps into the pool, and executes tasks and moves completed data into expected locations
    /// this can in theory be called as many times as possible. The more you step the faster the task pool can update/etc
    pub async fn step(&self) -> usize {
        // see if we have any futures that are done
        let data = self.data.read().await;
        let mut finished_tasks = Vec::with_capacity(data.allowed_at_once);
        for (task_id, join_handle) in data.running.iter() {
            if join_handle.is_finished() {
                finished_tasks.push(*task_id);
            }
        }
        drop(data);

        // move handle with its results from the running map, into the results map
        let mut data = self.data.write().await;
        for task_id in finished_tasks.into_iter() {
            let _ = data.running.remove_entry(&task_id);
            /*if let Some((task_id, task_handle)) = task_result {
                let task_result = task_handle.await;
                let result_entry = data.results.entry(task_id);
                match result_entry {
                    Entry::Occupied(_) => {
                        result_entry.and_modify(|v| *v = task_result);
                    }
                    Entry::Vacant(_) => {
                        result_entry.or_insert(task_result);
                    }
                };
            } */
        }
        drop(data);

        // start running futures if possible
        let mut data = self.data.write().await;
        if data.running.len() < data.allowed_at_once && !data.futures.is_empty() {
            'run_task: for _ in 0..(data.allowed_at_once - data.running.len()) {
                let task = data.futures.pop_front();
                if let Some((task_id, task)) = task {
                    data.running.insert(task_id, tokio::spawn(task()));
                } else {
                    break 'run_task; // nothing to add.
                }
            }
        }

        let total_running = data.running.len();
        drop(data);

        total_running
    }
}

/*
#[cfg(test)]
mod test {

    use crate::task_manager::*;
    use crate::util::unix_timestamp;
    use rand::{self, Rng};
    use std::time::Duration;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn pool() {
        let task_manager = TaskManager::new(25);
        tracing::info!("Created pool");

        let x = 100;
        let mut ids = Vec::new();
        for i in 0..x {
            let counter = i;
            ids.push(
                task_manager
                    .queue(Box::new(move || {
                        Box::pin(async move {
                            let num = rand::thread_rng().gen_range(1..10);
                            tracing::info!(
                                "Sleeping at: {} for iteration: {} , duration = {}",
                                unix_timestamp(),
                                counter,
                                num
                            );
                            tokio::time::sleep(Duration::from_secs(num)).await;
                            tracing::info!(
                                "Waking up from sleep at: {} for iteration: {}, duration = {}",
                                unix_timestamp(),
                                counter,
                                num
                            );
                        })
                    }))
                    .await,
            );
        }

        tracing::info!("Waiting 10 seconds before stepping");
        tokio::time::sleep(Duration::from_secs(10)).await;

        'task_loop: loop {
            tracing::info!("Sleeping! {}", unix_timestamp());
            tokio::time::sleep(Duration::from_secs(1)).await;
            tracing::info!("Stepping!");

            let running_currently = task_manager.step().await;
            tracing::info!("Running {} total task", running_currently);

            if task_manager.is_empty().await {
                break 'task_loop;
            }
        }

        tracing::info!("Done! No more tasks. ");
    }
}

*/
