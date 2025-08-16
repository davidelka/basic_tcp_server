use std::error::Error as StdError;
use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
};
use std::fmt::{Display, Formatter};
use log::error;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}


#[derive(Debug)]
pub enum ServerError {
    InternalError(String),
    OtherError(String),
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::InternalError(e) => write!(f, "{}", e),
            ServerError::OtherError(e) => write!(f, "{}", e)
        }
    }
}

impl StdError for ServerError {

}

type Job = Box<dyn FnOnce() -> Result<(), ServerError> + Send + 'static>;

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        if size == 0 {
            panic!("Size should be greater than 0");
        }

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
    F: FnOnce() -> Result<(), ServerError> + Send + 'static
    {
        let job = Box::new(f);
        let response = self.sender.as_ref();
        match response {
            Some(response) => {
                let _ = response.send(job).map_err(|e| error!("Error sending job to worker {e}"));
            },
            None => {
                error!("Error sending job to worker");
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.take() {
            drop(sender);
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                if let Err(e) = thread.join() {
                    error!("Worker {} panicked: {:?}", worker.id, e);
                } else {
                    log::info!(" worker id {} terminated", worker.id);
                }
            }
        }
    }
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = match receiver.lock() {
                Ok(guard) => guard.recv(),
                Err(poisoned) => {
                    error!("Worker {}: mutex poisoned, shutting down: {}", id, poisoned);
                    break; // exit the worker loop
                }
            };

            match message {
                Ok(job) => {
                    log::info!("worker ID: {} starting a job", id);
                    if let Err(e) = job() {
                        error!("Job failed: {}", e);
                    }
                }
                Err(_) => {
                    log::info!("worker thread terminated ID: {}", id);
                    break;
                }
            }

        });
        Worker{
            id,
            thread: Some(thread),
        }
    }
}