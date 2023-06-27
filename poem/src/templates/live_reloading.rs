use notify::{
    Watcher as WatcherTrait,
    Event, EventKind,
    RecursiveMode
};

use std::sync::{
    Arc, atomic::{ AtomicBool, Ordering },
};

pub(crate) struct Watcher {
    pub(crate) needs_reload: Arc<AtomicBool>,
    pub(crate) _path: String,
    _watcher: Option<Arc<dyn WatcherTrait + Send + Sync + 'static>>
}

impl Watcher {
    pub(crate) fn new(path: String) -> Self {
        let needs_reload = Arc::new(AtomicBool::new(false));
        let needs_reload_cloned = needs_reload.clone();

        let watcher = notify::recommended_watcher(move |event| match event {
            Ok(Event {
                kind:
                    EventKind::Create(_)
                    | EventKind::Modify(_)
                    | EventKind::Remove(_),
                ..
            }) => {
                needs_reload.store(true, Ordering::Relaxed);
                tracing::debug!("Sent reload request");
            },
            Err(e) => {
                // Ignore errors for now and just output them.
                // todo: make panic?
                tracing::debug!("Watcher error: {e:?}");
            },
            _ => {},
        });

        let watcher = watcher
            .map(|mut w| w
                .watch(std::path::Path::new(&path), RecursiveMode::Recursive)
                .map(|_| w)
            );

        let watcher = match watcher {
            Ok(Ok(w)) => {
                tracing::info!("Watching templates directory `{path}` for changes.");

                Some(Arc::new(w) as Arc<dyn WatcherTrait + Send + Sync>)
            }
            Err(e) | Ok(Err(e)) => {
                tracing::error!("Failed to start watcher: {e}");
                tracing::debug!("Watcher error: {e:?}");

                None
            },
        };

        Self {
            needs_reload: needs_reload_cloned,
            _path: path,
            _watcher: watcher,
        }
    }

    pub(crate) fn needs_reload(&self) -> bool {
        self.needs_reload.swap(false, Ordering::Relaxed)
    }
}

pub enum LiveReloading {
    Enabled(String),
    Debug(String),
    Disabled
}