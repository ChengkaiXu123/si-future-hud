//! Facts that differ by operating system, stated for every one of them.
//! A descriptor holds `PerOs` with three required fields, so leaving an
//! operating system out does not compile.

/// The operating systems Agent HUD runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Windows,
    Linux,
}

impl Os {
    /// Its lowercase name, as Rust's target constants spell it: "macos", "windows", "linux".
    pub fn name(self) -> &'static str {
        match self {
            Os::MacOs => "macos",
            Os::Windows => "windows",
            Os::Linux => "linux",
        }
    }
}

/// One fact for each operating system.
pub struct PerOs<T: 'static> {
    pub macos: Support<T>,
    pub windows: Support<T>,
    pub linux: Support<T>,
}

impl<T> PerOs<T> {
    pub fn on(&self, os: Os) -> &Support<T> {
        match os {
            Os::MacOs => &self.macos,
            Os::Windows => &self.windows,
            Os::Linux => &self.linux,
        }
    }
}

/// How far a fact holds on one operating system. Unsupported behaviour is
/// never attempted; unverified behaviour reads but does not write outside
/// Agent HUD until it is verified or the user opts in.
pub enum Support<T: 'static> {
    Verified(T),
    Unverified(T),
    /// Why it does not run on this operating system.
    Unsupported(&'static str),
}

impl<T> Support<T> {
    /// The fact, unless the behaviour is unsupported here.
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Verified(value) | Self::Unverified(value) => Some(value),
            Self::Unsupported(_) => None,
        }
    }

    /// Whether the behaviour may write outside Agent HUD, such as installing
    /// hooks or patching the client's settings, without the user opting in.
    pub fn writes(&self) -> bool {
        matches!(self, Self::Verified(_))
    }

    /// How the capability report words it.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Verified(_) => "live-verified",
            Self::Unverified(_) => "unverified",
            Self::Unsupported(_) => "unsupported",
        }
    }
}
