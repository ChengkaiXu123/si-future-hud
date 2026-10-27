//! A hook callback's caller: the processes it runs under, as the kernel
//! reports them, and the callback's own environment. Platform gathers these
//! facts the OS's way; each client's rules here say where a session started
//! and whether anyone watches it. A caller's own claim about its host is
//! never trusted, and a parent's environment is never read.
use serde_json::Value;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// A process a hook callback runs under.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ancestor {
    /// Its executable, as the kernel reports it.
    pub executable: PathBuf,
    /// The same path with links resolved, when it resolves.
    pub resolved: Option<PathBuf>,
    pub arguments: Vec<String>,
    /// The application bundle identifier, on systems with bundles.
    pub bundle: Option<String>,
}

/// What platform knows about a hook callback's caller.
pub struct Caller<'a> {
    /// The parent processes, nearest first, with shells left out.
    pub ancestors: &'a [Ancestor],
    /// The callback's own environment.
    pub env: &'a HashMap<String, String>,
    /// Whether a path is the client's installed program, links resolved.
    pub installed: &'a dyn Fn(&Path) -> bool,
}

impl Caller<'_> {
    pub fn env(&self, key: &str) -> Option<&str> {
        self.env.get(key).map(String::as_str)
    }
    /// Whether any caller is the app with this bundle identifier.
    pub fn under_bundle(&self, bundle: &str) -> bool {
        self.ancestors
            .iter()
            .any(|process| process.bundle.as_deref() == Some(bundle))
    }
}

/// Where a session's hooks come from.
#[derive(Clone, Debug, PartialEq)]
pub struct Origin {
    /// `cli`, `desktop`, `cowork`, `daemon`, `unknown` or `unsupported`.
    pub host: &'static str,
    /// Where the HUD can take the user back to the session.
    pub target: Value,
    /// Whether the session runs in an interactive terminal, so the terminal
    /// pane can be its target.
    pub interactive: bool,
    /// Whether the client's native channel knows the terminal the session
    /// runs in, as Codex's daemon does.
    pub ask_channel: bool,
}

impl Origin {
    /// A host that cannot be verified.
    pub fn unsupported() -> Self {
        Self {
            host: "unsupported",
            target: Value::Null,
            interactive: false,
            ask_channel: false,
        }
    }
}

/// A client's rule for where a session started, given the hook's caller and
/// the session's raw id.
pub type OriginRule = fn(&Caller, &str) -> Origin;
/// A client's rule for whether a hook comes from a run nobody watches, whose
/// requests stay with the client.
pub type UnattendedRule = fn(&Caller) -> bool;

/// A native executable name. Windows paths may also appear in imported facts
/// on another host, so recognize both separators and the native `.exe` suffix.
pub(crate) fn native_program(path: &Path, name: &str) -> bool {
    let Some(path) = path.to_str() else {
        return false;
    };
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file == name
        || file.rsplit_once('.').is_some_and(|(stem, extension)| {
            extension.eq_ignore_ascii_case("exe") && stem.eq_ignore_ascii_case(name)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_names_cover_windows_without_matching_wrappers() {
        assert!(native_program(Path::new("/usr/local/bin/codex"), "codex"));
        assert!(native_program(
            Path::new(r"C:\Program Files\Codex\CODEX.EXE"),
            "codex"
        ));
        assert!(!native_program(Path::new(r"C:\bin\codex.cmd"), "codex"));
        assert!(!native_program(Path::new("/bin/not-codex"), "codex"));
        assert!(!native_program(Path::new("/bin/CODEX"), "codex"));
    }
}
