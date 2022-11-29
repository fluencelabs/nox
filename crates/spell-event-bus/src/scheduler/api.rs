use fluence_libp2p::types::Outlet;
use futures::channel::mpsc::TrySendError;
use std::fmt;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug)]
pub struct TimerConfig {
    pub period: Duration,
}

#[derive(Debug)]
pub enum Command {
    AddSpell { id: String, config: TimerConfig },
    RemoveSpell { id: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    TimeTrigger { id: String },
}

#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("Can't add spell {0} to the scheduler")]
    CommandAddSpellError(String),
    #[error("Can't remove spell {0} from the scheduler")]
    CommandRemoveSpellError(String),
}

#[derive(Clone)]
pub struct SchedulerApi {
    send_command: Outlet<Command>,
}

impl fmt::Debug for SchedulerApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SchedulerApi").finish()
    }
}

impl SchedulerApi {
    pub fn new(send_command: Outlet<Command>) -> Self {
        Self { send_command }
    }

    fn send(&self, command: Command) -> Result<(), TrySendError<Command>> {
        self.send_command.unbounded_send(command)
    }

    pub fn add(&self, id: String, config: TimerConfig) -> Result<(), SchedulerError> {
        self.send(Command::AddSpell {
            id: id.clone(),
            config,
        })
        .map_err(|_| SchedulerError::CommandAddSpellError(id))
    }

    pub fn remove(&self, id: String) -> Result<(), SchedulerError> {
        self.send(Command::RemoveSpell { id: id.clone() })
            .map_err(|_| SchedulerError::CommandRemoveSpellError(id))
    }
}
