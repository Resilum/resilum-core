use std::sync::Mutex;

static HANDED_OVER: Mutex<bool> = Mutex::new(false);

pub(super) fn hand_it_to_the_radio() {
    let mut done = HANDED_OVER.lock().unwrap_or_else(|held| held.into_inner());
    if *done {
        return;
    }
    let vm = ndk_context::android_context().vm();
    if vm.is_null() {
        tracing::warn!("no jvm was installed before the radio was opened");
        return;
    }
    blew::platform::android::init_jvm(unsafe { jni::JavaVM::from_raw(vm.cast()) });
    *done = true;
}
