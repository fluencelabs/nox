pub mod api;

use crate::scheduler::api::*;
use async_std::task::JoinHandle;
use fluence_libp2p::types::Inlet;
use futures::{channel::mpsc::unbounded, select, StreamExt};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Periodic<T> {
    pub id: T,
    pub period: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Scheduled<T> {
    data: Periodic<T>,
    // the time after we need to run the task
    run_at: Instant,
}

impl<T: Eq> Scheduled<T> {
    fn new(data: Periodic<T>, now: Instant) -> Scheduled<T> {
        let run_at = now.checked_add(data.period).expect("time overflow?");
        Scheduled { data, run_at }
    }

    fn reschedule(mut self, now: Instant) -> Scheduled<T> {
        self.run_at = now.checked_add(self.data.period).expect("time overflow?");
        self
    }
}

// Implement it this way for min heap
impl<T: PartialEq + Eq> Ord for Scheduled<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.run_at.cmp(&self.run_at)
    }
}

impl<T: Eq> PartialOrd for Scheduled<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct SchedulerConfig {
    pub timer_resolution: Duration,
}

pub struct Scheduler {
    timer_resolution: Duration,
    recv_command: Inlet<Command>,
    // what to run when its time to execute the task
    callback: Box<dyn Fn(&str) + Send>,
}

impl Scheduler {
    pub fn new(
        config: SchedulerConfig,
        callback: impl Fn(&str) + Send + 'static,
    ) -> (Self, SchedulerApi) {
        let (send, recv) = unbounded();
        let api = SchedulerApi::new(send);
        let this = Self {
            timer_resolution: config.timer_resolution,
            recv_command: recv,
            callback: Box::new(callback),
        };
        (this, api)
    }

    pub fn set_callback(&mut self, callback: impl Fn(&str) + Send + 'static) {
        self.callback = Box::new(callback);
    }

    pub fn start(self) -> JoinHandle<()> {
        async_std::task::spawn(async move {
            let timer_resolution = self.timer_resolution;
            let mut command_channel = self.recv_command.fuse();
            let mut timer = async_std::stream::interval(timer_resolution).fuse();
            let mut heap: BinaryHeap<Scheduled<String>> = BinaryHeap::new();
            let callback = self.callback;
            loop {
                select!(
                    _ = timer.select_next_some() => {
                        let now = Instant::now();
                        while let Some(task) = heap.peek() {
                            if task.run_at > now {
                                break;
                            }
                            let task = heap.pop().unwrap();
                            log::debug!("Executing task with id: {}", task.data.id);
                            callback(&task.data.id);
                            heap.push(task.reschedule(now));
                        }
                    },
                    command = command_channel.select_next_some() => {
                        log::debug!("Received a command: {:?}", command);
                        match command {
                            Command::Add { id , config } => {
                                let periodic = Periodic { id, period: config.period };
                                heap.push(Scheduled::new(periodic, Instant::now()))
                            },
                            Command::Remove { id } => {
                                heap.retain(|scheduled| scheduled.data.id != id);
                            },
                        }
                    }
                );
            }
        })
    }
}

#[test]
fn test1() {
    use async_std::task;
    let (scheduler, api) = Scheduler::new(
        SchedulerConfig {
            timer_resolution: Duration::from_secs(1),
        },
        |id| println!("{:?}", id),
    );
    let _ = scheduler.start();

    api.add(
        "spell1".to_string(),
        TimerConfig {
            period: Duration::from_secs(1),
        },
    )
    .unwrap();
    api.add(
        "spell2".to_string(),
        TimerConfig {
            period: Duration::from_secs(3),
        },
    )
    .unwrap();
    task::block_on(async { task::sleep(Duration::from_secs(10)).await });

    println!("remove spell2");
    api.remove("spell2".to_string()).unwrap();
    task::block_on(async { task::sleep(Duration::from_secs(10)).await });

    println!("add spell3");
    api.add(
        "spell3".to_string(),
        TimerConfig {
            period: Duration::from_secs(4),
        },
    )
    .unwrap();
    task::block_on(async { task::sleep(Duration::from_secs(10)).await });
}
