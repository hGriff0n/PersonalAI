extern crate tokio;

use std::sync::mpsc;

use tokio::prelude::{Async, Future};

pub struct Canceller {
    pub rx: mpsc::Receiver<()>,
}

impl Future for Canceller {
    type Item = ();
    type Error = mpsc::RecvError;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        match self.rx.try_recv() {
            Ok(_) => Ok(Async::Ready(())),
            Err(_) => Ok(Async::NotReady)
        }
    }
}
