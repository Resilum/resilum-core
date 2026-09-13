use std::fmt::Display;

use tokio::sync::{broadcast, mpsc, watch};

pub trait NoOneIsListening {
    fn no_one_is_listening(self);
}

impl<T, U> NoOneIsListening for Result<T, mpsc::error::SendError<U>> {
    fn no_one_is_listening(self) {}
}

impl<T, U> NoOneIsListening for Result<T, broadcast::error::SendError<U>> {
    fn no_one_is_listening(self) {}
}

impl<T, U> NoOneIsListening for Result<T, watch::error::SendError<U>> {
    fn no_one_is_listening(self) {}
}

impl<T, U> NoOneIsListening for Result<T, std::sync::mpsc::SendError<U>> {
    fn no_one_is_listening(self) {}
}

pub trait TheQueueMayBeFull {
    fn dropped_if_the_queue_is_full(self, what: &str);
}

impl<T, U> TheQueueMayBeFull for Result<T, mpsc::error::TrySendError<U>> {
    fn dropped_if_the_queue_is_full(self, what: &str) {
        if let Err(mpsc::error::TrySendError::Full(_)) = self {
            tracing::debug!(what, "dropped, the queue it goes into is full");
        }
    }
}

pub trait OnTheWayOut {
    fn on_the_way_out(self, what: &str);
}

impl<T, E: Display> OnTheWayOut for Result<T, E> {
    fn on_the_way_out(self, what: &str) {
        if let Err(why) = self {
            tracing::debug!(what, error = %why, "failed while shutting down");
        }
    }
}

pub trait ItWasAlreadyThere {
    fn it_was_already_there(self);
}

impl<T, E> ItWasAlreadyThere for Result<T, E> {
    fn it_was_already_there(self) {}
}

pub trait ItOnlyMustNotPanic {
    fn it_only_must_not_panic(self);
}

impl<T> ItOnlyMustNotPanic for T {
    fn it_only_must_not_panic(self) {}
}
