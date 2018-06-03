extern crate tokio;

use std;
use std::sync::mpsc;

use tokio::prelude::{Async, Future};

pub struct FutureChannel<T> {
    rx: mpsc::Receiver<T>,
}

impl<T: std::fmt::Debug> FutureChannel<T> {
    pub fn new(rx: mpsc::Receiver<T>) -> Self {
        Self{ rx: rx }
    }
}

impl<T: std::fmt::Debug> Future for FutureChannel<T> {
    type Item = T;
    type Error = mpsc::RecvError;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        match self.rx.try_recv() {
            Ok(val) => Ok(Async::Ready(val)),
            Err(_) => Ok(Async::NotReady)
        }
    }
}
