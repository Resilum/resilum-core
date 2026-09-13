use tokio::task::JoinError;

pub(crate) fn report(name: &str, ending: Result<(), JoinError>) {
    match ending {
        Ok(()) => tracing::info!(task = %name, "ended"),
        Err(join) if join.is_cancelled() => tracing::debug!(task = %name, "cancelled"),
        Err(join) => tracing::error!(task = %name, reason = %reason(join.into_panic()), "panicked"),
    }
}

pub(crate) fn reason(payload: Box<dyn std::any::Any + Send>) -> String {
    of_the_payload(payload.as_ref())
}

pub(crate) fn of_the_payload(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        return (*text).to_owned();
    }
    if let Some(text) = payload.downcast_ref::<String>() {
        return text.clone();
    }
    "a panic payload that is neither &str nor String".to_owned()
}

#[cfg(test)]
mod tests;
