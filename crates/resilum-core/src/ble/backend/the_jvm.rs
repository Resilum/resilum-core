use std::sync::Once;

static HANDED_OVER: Once = Once::new();

pub(super) fn hand_it_to_the_radio() {
    HANDED_OVER.call_once(|| {
        let context = ndk_context::android_context();
        let vm = context.vm();
        if vm.is_null() {
            tracing::warn!("no jvm was installed before the radio was opened");
            return;
        }
        blew::platform::android::init_jvm(unsafe { jni::JavaVM::from_raw(vm.cast()) });
    });
}
