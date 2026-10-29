//! Native filesystem events only signal the serial collector, as FileChangeMonitor
//! does in the Swift app. No callback reads logs or mutates session state.
use agent_hud_core::SourceRoot;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
};

type Waker = Arc<dyn Fn() + Send + Sync>;
pub struct FileChanges {
    watcher: Option<RecommendedWatcher>,
    roots: Vec<PathBuf>,
    watched: HashSet<PathBuf>,
    changes: Arc<Mutex<Option<HashSet<PathBuf>>>>,
    names: Arc<Mutex<Vec<PathBuf>>>,
    waker: Arc<Mutex<Option<Waker>>>,
    working: bool,
}
impl FileChanges {
    pub fn new(roots: &[SourceRoot]) -> Self {
        let changes: Arc<Mutex<Option<HashSet<PathBuf>>>> = Arc::new(Mutex::new(None));
        let waker: Arc<Mutex<Option<Waker>>> = Arc::new(Mutex::new(None));
        let names = Arc::new(Mutex::new(root_names(roots.iter().map(|root| &root.path))));
        let pending = Arc::clone(&changes);
        let notify_wake = Arc::clone(&waker);
        let source_names = Arc::clone(&names);
        let watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            if matches!(&event,Ok(event) if matches!(event.kind,EventKind::Access(_)) && !event.need_rescan()) {
                return;
            }
            // A file root is watched through its parent. In particular, the Codex
            // index must not wake the collector for every unrelated .codex write.
            let changed = match event {
                Ok(event) if !event.need_rescan() && !event.paths.is_empty() => {
                    let Ok(roots) = source_names.lock() else { return; };
                    let mut owned = HashSet::new();
                    for path in event.paths {
                        for root in roots.iter() {
                            if path.starts_with(root) { owned.insert(path.clone()); }
                            else if root.starts_with(&path) { owned.insert(root.clone()); }
                        }
                    }
                    if owned.is_empty() { return; }
                    Some(owned)
                }
                _ => None,
            };
            if let Ok(mut paths) = pending.lock() {
                match changed {
                    Some(changed) => {
                        if let Some(known) = paths.as_mut() {
                            known.extend(changed);
                            if known.len() > 10_000 {
                                *paths = None;
                            }
                        }
                    }
                    _ => *paths = None,
                }
            }
            let wake = notify_wake.lock().ok().and_then(|wake| wake.clone());
            if let Some(wake) = wake {
                wake();
            }
        })
        .ok();
        let mut this = Self {
            watcher,
            roots: roots.iter().map(|root| root.path.clone()).collect(),
            watched: HashSet::new(),
            changes,
            names,
            waker,
            working: false,
        };
        this.update();
        this
    }
    pub fn set_waker(&mut self, wake: Waker) {
        if let Ok(mut waker) = self.waker.lock() {
            *waker = Some(wake);
        }
    }
    fn update(&mut self) {
        let all = self
            .roots
            .iter()
            .filter(|path| path.exists())
            .map(|path| {
                if path.is_file() {
                    path.parent().unwrap_or(path).to_owned()
                } else {
                    path.clone()
                }
            })
            .collect::<HashSet<_>>();
        let existing = all
            .iter()
            .filter(|path| {
                !all.iter()
                    .any(|parent| *path != parent && path.starts_with(parent))
            })
            .cloned()
            .collect::<HashSet<_>>();
        if existing == self.watched {
            return;
        }
        if let Ok(mut names) = self.names.lock() {
            *names = root_names(self.roots.iter());
        }
        self.working = false;
        if let Some(watcher) = &mut self.watcher {
            // Local build: one change for every root. macOS's watcher stops and
            // restarts its event stream for each path watched one at a time,
            // which took seven seconds for a few dozen roots at launch.
            let removed = self
                .watched
                .difference(&existing)
                .cloned()
                .collect::<Vec<_>>();
            let added = existing
                .difference(&self.watched)
                .cloned()
                .collect::<Vec<_>>();
            let mut paths = watcher.paths_mut();
            for path in &removed {
                let _ = paths.remove(path);
            }
            let mut watching = Vec::new();
            for path in added {
                if paths.add(&path, RecursiveMode::Recursive).is_ok() {
                    watching.push(path);
                }
            }
            let committed = paths.commit().is_ok();
            self.watched.retain(|path| existing.contains(path));
            if committed {
                self.watched.extend(watching);
            }
            self.working = committed && self.watched == existing && !existing.is_empty();
        }
        if let Ok(mut paths) = self.changes.lock() {
            *paths = None;
        }
    }
    pub fn consume(&mut self) -> Option<HashSet<PathBuf>> {
        self.update();
        if !self.working {
            return None;
        }
        self.changes
            .lock()
            .ok()
            .and_then(|mut paths| (*paths).replace(HashSet::new()))
    }
}

fn root_names<'a>(roots: impl Iterator<Item = &'a PathBuf>) -> Vec<PathBuf> {
    let mut names = HashSet::new();
    for root in roots {
        names.insert(root.clone());
        if let Ok(real) = root.canonicalize() {
            names.insert(real);
        }
    }
    names.into_iter().collect()
}
