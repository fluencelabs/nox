pub mod api;

use crate::scheduler::api::*;
use async_std::task::JoinHandle;
use eyre::eyre;
use fluence_libp2p::types::{Inlet, Outlet};
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
    fn new(data: Periodic<T>, now: Instant) -> eyre::Result<Scheduled<T>> {
        let run_at = now
            .checked_add(data.period)
            .ok_or_else(|| eyre!("Timestamp overflow"))?;
        Ok(Scheduled { data, run_at })
    }

    fn reschedule(mut self, now: Instant) -> eyre::Result<Scheduled<T>> {
        self.run_at = now
            .checked_add(self.data.period)
            .ok_or_else(|| eyre!("Timestamp overflow"))?;
        Ok(self)
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
    spell_events_out: Outlet<Event>,
}

impl Scheduler {
    pub fn new(config: SchedulerConfig) -> (Self, SchedulerApi, Inlet<Event>) {
        let (send, recv) = unbounded();
        let (spell_events_out, spell_events_in) = unbounded();

        let api = SchedulerApi::new(send);
        let this = Self {
            timer_resolution: config.timer_resolution,
            recv_command: recv,
            spell_events_out,
        };
        (this, api, spell_events_in)
    }

    pub fn start(self) -> JoinHandle<()> {
        async_std::task::spawn(async move {
            let timer_resolution = self.timer_resolution;
            let mut command_channel = self.recv_command.fuse();
            let spell_events_out = self.spell_events_out.clone();

            let mut timer = async_std::stream::interval(timer_resolution).fuse();
            let mut heap: BinaryHeap<Scheduled<String>> = BinaryHeap::new();
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
                            match spell_events_out.unbounded_send(Event::TimeTrigger { id: task.data.id.clone() }) {
                                Err(err) => {
                                    let err_msg = format!("{:?}", err);
                                    let msg = err.into_inner();
                                    log::warn!("unable to send event {:?} to sorcerer: {:?}", msg, err_msg)
                                }
                                Ok(_v) => {}
                            };
                            match task.reschedule(now) {
                                Ok(t) => heap.push(t),
                                Err(e) => log::error!("Can't reschedule task: {:?}", e),
                            }

                        }
                    },
                    command = command_channel.select_next_some() => {
                        log::debug!("Received a command: {:?}", command);
                        match command {
                            Command::AddSpell { id , config } => {
                                let periodic = Periodic { id, period: config.period };
                                match Scheduled::new(periodic, Instant::now()) {
                                    Ok(t) => heap.push(t),
                                    Err(e) => log::error!("Can't schedule task: {:?}", e),
                                }
                            },
                            Command::RemoveSpell { id } => {
                                heap.retain(|scheduled| scheduled.data.id != id);
                            },
                        }
                    }
                );
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use std::cmp::max;
    use std::ops::{Add, Mul};
    use std::time::Duration;

    use crate::scheduler::api::{Event, TimerConfig};
    use crate::scheduler::{Scheduler, SchedulerConfig};

    #[test]
    fn scheduler_add_remove_test() {
        use async_std::task;
        let timer_resolution = Duration::from_millis(1);
        let (scheduler, api, event_stream) = Scheduler::new(SchedulerConfig { timer_resolution });
        scheduler.start();

        let spell1_id = "spell1".to_string();
        let spell2_id = "spell2".to_string();
        let spell1_period = Duration::from_millis(5);
        let spell2_period = Duration::from_millis(8);
        api.add(
            spell1_id.clone(),
            TimerConfig {
                period: spell1_period,
            },
        )
        .unwrap();
        api.add(
            spell2_id.clone(),
            TimerConfig {
                period: spell2_period,
            },
        )
        .unwrap();

        // let's wait for both spell to be executed once
        task::block_on(async { task::sleep(max(spell1_period, spell2_period)).await });

        // let's remove spell1"
        api.remove(spell1_id.clone()).unwrap();

        // let's collect events
        let events = task::block_on(async { event_stream.take(3).collect::<Vec<Event>>().await });
        assert_eq!(events.len(), 3);
        assert_eq!(
            events[0],
            Event::TimeTrigger {
                id: spell1_id.clone()
            }
        );
        assert_eq!(
            events[1],
            Event::TimeTrigger {
                id: spell2_id.clone()
            }
        );
        assert_eq!(events[2], Event::TimeTrigger { id: spell2_id });
    }
}
