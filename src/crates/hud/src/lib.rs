//! What the HUD shows, decided once for every window, the menu bar and MCP:
//! the panel's saved layout and the rules an agent's draft must meet
//! (`presentation`), what each agent's Bot does (`bots`) and the vendor rows a
//! HUD strip draws (`vendors`). Nothing here opens a window or names an
//! operating system; the host (`agent-hud-host`) puts it on screen.
pub mod bots;
pub mod presentation;
pub mod vendors;
