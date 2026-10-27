/// Local build: not yet on Linux. SI Future HUD ships for macOS only, so no
/// installed location is delegated here and MCP setup and client hooks stay
/// as upstream has them. An adapter would return the installer's fixed
/// program path, as the macOS file returns the bundle in Applications.
pub fn installed_executable() -> Option<std::path::PathBuf> {
    None
}
