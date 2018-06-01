extern crate tokio;

use std;
use std::io;
use std::sync::mpsc;

use tokio::prelude::{Async, Future, Stream};
use tokio::prelude::stream::Map;

pub struct FutureChannel<T> {
    rx: mpsc::Receiver<T>,
}

impl<T: std::fmt::Debug> FutureChannel<T> {
    pub fn new(rx: mpsc::Receiver<T>) -> Self {
        Self{ rx: rx }
    }

    pub fn transform<U, F>(self, f: F) -> Map<Self, F> where F: FnMut(T) -> U {
        Stream::map(self, f)
    }
}

impl<T: std::fmt::Debug> Future for FutureChannel<T> {
    type Item = T;
    type Error = mpsc::RecvError;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        let val = self.rx.try_recv();
        println!("poll {:?}", val);
        match val {
            Ok(val) => Ok(Async::Ready(val)),
            Err(_) => Ok(Async::NotReady)
        }
    }
}


impl<T: std::fmt::Debug> Stream for FutureChannel<T> {
    type Item = T;
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        let val = self.rx.try_recv();
        println!("poll {:?}", val);
        match val {
            Ok(val) => Ok(Async::Ready(Some(val))),
            Err(_) => Ok(Async::NotReady)
        }
    }
}
