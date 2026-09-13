use std::io;
use std::thread::JoinHandle;

pub fn a_thread_of_its_own<F, T>(name: impl Into<String>, run: F) -> io::Result<JoinHandle<T>>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    std::thread::Builder::new().name(name.into()).spawn(run)
}
