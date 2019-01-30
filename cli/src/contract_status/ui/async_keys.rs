use std::io;
use std::sync::mpsc;
use std::thread;
use termion::event::Key;
use termion::input::TermRead;

#[derive(Debug)]
pub enum AsyncKey {
    Input(Key),
    Empty,
}

pub struct AsyncKeys {
    rx: mpsc::Receiver<AsyncKey>,
}

impl AsyncKeys {
    pub fn new() -> AsyncKeys {
        let (tx, rx) = mpsc::channel();

        let key_tx = tx.clone();
        thread::spawn(move || {
            let stdin = io::stdin();
            for event in stdin.keys() {
                match event {
                    Ok(key) => {
                        if key_tx.send(AsyncKey::Input(key)).is_err() {
                            return;
                        }
                    }
                    _ => {
                        key_tx.send(AsyncKey::Empty).unwrap();
                    }
                }
            }
        });

        AsyncKeys { rx }
    }

    pub fn next(&self) -> Result<AsyncKey, mpsc::RecvError> {
        self.rx.recv()
    }
}
