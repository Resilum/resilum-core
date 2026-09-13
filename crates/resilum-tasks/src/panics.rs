use std::backtrace::Backtrace;
use std::panic::PanicHookInfo;

use crate::ending;

pub fn are_told_to_the_log() {
    std::panic::set_hook(Box::new(tell));
}

fn tell(panicked: &PanicHookInfo<'_>) {
    let where_it_happened = panicked
        .location()
        .map_or_else(|| "somewhere unrecorded".to_owned(), ToString::to_string);
    tracing::error!(
        thread = std::thread::current().name().unwrap_or("unnamed"),
        at = %where_it_happened,
        reason = %ending::of_the_payload(panicked.payload()),
        backtrace = %Backtrace::force_capture(),
        "panicked"
    );
}
