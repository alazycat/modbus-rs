#[cfg(all(test, feature = "tracing"))]
pub mod test_trace {
    //! Test helper for capturing `tracing` events in unit tests.

    use alloc::string::String;
    use alloc::vec::Vec;
    use core::fmt::Debug;
    use std::sync::{Arc, Mutex};

    use tracing::field::Field;
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::registry::LookupSpan;

    /// A recorded tracing event captured by [`TraceRecorder`].
    #[derive(Clone, Debug)]
    pub struct EventRecord {
        pub name: String,
        pub fields: Vec<(String, String)>,
    }

    /// A `tracing` `Layer` that records every event it sees.
    #[derive(Default, Clone)]
    pub struct TraceRecorder {
        events: Arc<Mutex<Vec<EventRecord>>>,
    }

    impl TraceRecorder {
        /// Create a new, empty recorder.
        pub fn new() -> Self {
            Self::default()
        }

        /// Return a snapshot of the events recorded so far.
        pub fn events(&self) -> Vec<EventRecord> {
            self.events.lock().unwrap().clone()
        }

        /// Drain and return the events recorded so far.
        pub fn take(&self) -> Vec<EventRecord> {
            std::mem::take(&mut *self.events.lock().unwrap())
        }
    }

    struct FieldVisitor(Vec<(String, String)>);

    impl tracing::field::Visit for FieldVisitor {
        fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
            self.0.push((field.name().to_string(), format!("{value:?}")));
        }
    }

    impl<S> Layer<S> for TraceRecorder
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut visitor = FieldVisitor(Vec::new());
            event.record(&mut visitor);
            self.events.lock().unwrap().push(EventRecord {
                name: event.metadata().name().to_string(),
                fields: visitor.0,
            });
        }
    }

    /// Install `recorder` as the default subscriber for the duration of `f`.
    pub fn with_default<F, R>(recorder: &TraceRecorder, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        use std::sync::Once;
        use tracing_subscriber::filter::LevelFilter;
        use tracing_subscriber::layer::SubscriberExt;

        static INIT_GLOBAL: Once = Once::new();
        INIT_GLOBAL.call_once(|| {
            use tracing_subscriber::util::SubscriberInitExt;
            let _ = tracing_subscriber::registry()
                .with(LevelFilter::TRACE)
                .try_init();
        });

        let subscriber = tracing_subscriber::registry()
            .with(recorder.clone())
            .with(LevelFilter::TRACE);
        let _guard = tracing::subscriber::set_default(subscriber);
        tracing::callsite::rebuild_interest_cache();
        f()
    }
}
