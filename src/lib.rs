// SPDX-License-Identifier: Apache-2.0
//! SigmaOS-polybar — SigmaOS polybar component inspired by Linux Mint / Omarchy / Arch Linux.
//!
//! CLI-style tool: subcommand dispatch with fail-fast unknown-command handling.
//! Principles: single responsibility, fail-fast, defensive programming,
//! component isolation, stable API, deterministic tests.
#![forbid(unsafe_code)]

use std::fmt;
/// Errors for `ToolError` operations. Fail-fast: invalid input is rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    /// Tool name must not be empty
    EmptyName,
    /// Unrecognized subcommand
    UnknownCommand(String),
    /// Subcommand requires arguments
    MissingArgs(String),
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "tool name must not be empty"),
            Self::UnknownCommand(x) => write!(f, "unrecognized subcommand: {x}"),
            Self::MissingArgs(x) => write!(f, "subcommand requires arguments: {x}"),
        }
    }
}

impl std::error::Error for ToolError {}

/// A command-line tool with a small fixed subcommand set.
#[derive(Debug)]
pub struct Tool {
    name: String,
}

impl Tool {
    /// Create the tool; fails fast on empty name.
    pub fn new(name: impl Into<String>) -> Result<Self, ToolError> {
        let name = name.into();
        if name.trim().is_empty() { return Err(ToolError::EmptyName); }
        Ok(Self { name })
    }

    /// Tool name.
    pub fn name(&self) -> &str { &self.name }

    /// Dispatch a subcommand. Supported: `status`, `version`, `help`.
    /// Commands needing args that get none fail fast.
    pub fn run(&self, args: &[&str]) -> Result<String, ToolError> {
        match args.first().copied().unwrap_or("help") {
            "status" => Ok(format!("{}: ready", self.name)),
            "version" => Ok(format!("{}: 0.1.0", self.name)),
            "help" => Ok(format!("{}: commands: status, version, help", self.name)),
            other => Err(ToolError::UnknownCommand(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_and_version() {
        let t = Tool::new("demo").unwrap();
        assert_eq!(t.run(&["status"]).unwrap(), "demo: ready");
        assert!(t.run(&["version"]).unwrap().contains("0.1.0"));
    }

    #[test]
    fn no_args_shows_help() {
        let t = Tool::new("demo").unwrap();
        assert!(t.run(&[]).unwrap().contains("commands"));
    }

    #[test]
    fn unknown_command_fails() {
        let t = Tool::new("demo").unwrap();
        assert!(matches!(t.run(&["bogus"]), Err(ToolError::UnknownCommand(_))));
    }

    #[test]
    fn empty_name_fails() {
        assert!(Tool::new(" ").is_err());
    }

}

/// Backwards-compatible component handle (kept for existing callers/tests).
#[derive(Debug, Default)]
pub struct Component {
    name: &'static str,
}

impl Component {
    /// Create the component handle.
    pub fn new() -> Self {
        Self { name: "SigmaOS Component" }
    }

    /// Component label.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

// ─── Polybar domain model ─────────────────────────────────────────────

/// Bar position on screen edge
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarPosition { Top, Bottom, Left, Right }

/// Module kind for polybar
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolybarModuleKind {
    Clock, InternalPulseAudio, Network, XWorkspaces,
    Filesystem, Memory, Cpu, Backlight, Battery,
    Tray, Custom(String),
}

/// Module state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState { Enabled, Disabled }

/// Polybar module
#[derive(Debug, Clone)]
pub struct PolybarModule {
    pub name: String,
    pub kind: PolybarModuleKind,
    pub position: String, // left/center/right
    pub state: ModuleState,
    pub interval: u32,
}

impl PolybarModule {
    pub fn new(name: &str, kind: PolybarModuleKind, position: &str) -> Self {
        Self { name: name.into(), kind, position: position.into(), state: ModuleState::Enabled, interval: 5 }
    }
}

/// Polybar bar
#[derive(Debug, Clone)]
pub struct Polybar {
    pub monitor: String,
    pub position: BarPosition,
    pub height: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub modules: Vec<PolybarModule>,
    pub visible: bool,
}

impl Polybar {
    pub fn new(monitor: &str, position: BarPosition) -> Self {
        Self {
            monitor: monitor.into(),
            position,
            height: 30,
            offset_x: 0,
            offset_y: 0,
            modules: vec![],
            visible: true,
        }
    }

    pub fn add_module(&mut self, module: PolybarModule) -> bool {
        if self.modules.iter().any(|m| m.name == module.name) { return false; }
        self.modules.push(module);
        true
    }

    pub fn remove_module(&mut self, name: &str) -> bool {
        let before = self.modules.len();
        self.modules.retain(|m| m.name != name);
        self.modules.len() < before
    }

    pub fn modules_at(&self, position: &str) -> Vec<&PolybarModule> {
        self.modules.iter().filter(|m| m.position == position).collect()
    }

    pub fn enabled_modules(&self) -> Vec<&PolybarModule> {
        self.modules.iter().filter(|m| m.state == ModuleState::Enabled).collect()
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }
}

/// Polybar manager for multiple bars
#[derive(Debug, Default)]
pub struct PolybarManager {
    bars: Vec<Polybar>,
}

impl PolybarManager {
    pub fn new() -> Self { Self::default() }

    pub fn add_bar(&mut self, bar: Polybar) -> bool {
        if self.bars.iter().any(|b| b.monitor == bar.monitor) { return false; }
        self.bars.push(bar);
        true
    }

    pub fn get_bar(&self, monitor: &str) -> Option<&Polybar> {
        self.bars.iter().find(|b| b.monitor == monitor)
    }

    pub fn get_bar_mut(&mut self, monitor: &str) -> Option<&mut Polybar> {
        self.bars.iter_mut().find(|b| b.monitor == monitor)
    }

    pub fn remove_bar(&mut self, monitor: &str) -> bool {
        let before = self.bars.len();
        self.bars.retain(|b| b.monitor != monitor);
        self.bars.len() < before
    }

    pub fn list_bars(&self) -> &[Polybar] { &self.bars }

    pub fn bars_on_position(&self, pos: BarPosition) -> Vec<&Polybar> {
        self.bars.iter().filter(|b| b.position == pos).collect()
    }

    pub fn total_modules(&self) -> usize {
        self.bars.iter().map(|b| b.enabled_modules().len()).sum()
    }
}

#[cfg(test)]
mod polybar_tests {
    use super::*;

    #[test]
    fn test_bar_position() {
        let positions = vec![BarPosition::Top, BarPosition::Bottom, BarPosition::Left, BarPosition::Right];
        assert_eq!(positions.len(), 4);
    }

    #[test]
    fn test_module_kind() {
        let kinds = vec![
            PolybarModuleKind::Clock, PolybarModuleKind::InternalPulseAudio,
            PolybarModuleKind::Network, PolybarModuleKind::XWorkspaces,
            PolybarModuleKind::Filesystem, PolybarModuleKind::Memory,
            PolybarModuleKind::Cpu, PolybarModuleKind::Backlight,
            PolybarModuleKind::Battery, PolybarModuleKind::Tray,
        ];
        assert_eq!(kinds.len(), 10);
    }

    #[test]
    fn test_bar_creation() {
        let bar = Polybar::new("DP-1", BarPosition::Top);
        assert_eq!(bar.monitor, "DP-1");
        assert_eq!(bar.position, BarPosition::Top);
        assert_eq!(bar.height, 30);
        assert!(bar.visible);
    }

    #[test]
    fn test_add_remove_module() {
        let mut bar = Polybar::new("eDP-1", BarPosition::Bottom);
        let m = PolybarModule::new("clock", PolybarModuleKind::Clock, "right");
        assert!(bar.add_module(m));
        assert_eq!(bar.modules.len(), 1);
        assert!(!bar.add_module(PolybarModule::new("clock", PolybarModuleKind::Clock, "left")));
        assert!(bar.remove_module("clock"));
        assert!(bar.modules.is_empty());
    }

    #[test]
    fn test_module_positions() {
        let mut bar = Polybar::new("HDMI-A-1", BarPosition::Top);
        bar.add_module(PolybarModule::new("ws", PolybarModuleKind::XWorkspaces, "left"));
        bar.add_module(PolybarModule::new("clock", PolybarModuleKind::Clock, "right"));
        bar.add_module(PolybarModule::new("tray", PolybarModuleKind::Tray, "right"));
        
        assert_eq!(bar.modules_at("left").len(), 1);
        assert_eq!(bar.modules_at("right").len(), 2);
    }

    #[test]
    fn test_toggle_visibility() {
        let mut bar = Polybar::new("DP-2", BarPosition::Top);
        assert!(bar.visible);
        bar.toggle_visibility();
        assert!(!bar.visible);
        bar.toggle_visibility();
        assert!(bar.visible);
    }

    #[test]
    fn test_manager_add_get_remove() {
        let mut mgr = PolybarManager::new();
        assert!(mgr.add_bar(Polybar::new("DP-1", BarPosition::Top)));
        assert!(mgr.add_bar(Polybar::new("HDMI-A-1", BarPosition::Top)));
        assert!(!mgr.add_bar(Polybar::new("DP-1", BarPosition::Bottom))); // dup monitor
        
        assert!(mgr.get_bar("DP-1").is_some());
        assert!(mgr.remove_bar("HDMI-A-1"));
        assert_eq!(mgr.list_bars().len(), 1);
    }

    #[test]
    fn test_manager_positions_and_modules() {
        let mut mgr = PolybarManager::new();
        let mut b1 = Polybar::new("DP-1", BarPosition::Top);
        b1.add_module(PolybarModule::new("clock", PolybarModuleKind::Clock, "right"));
        let mut b2 = Polybar::new("DP-2", BarPosition::Bottom);
        b2.add_module(PolybarModule::new("net", PolybarModuleKind::Network, "left"));
        b2.add_module(PolybarModule::new("audio", PolybarModuleKind::InternalPulseAudio, "right"));
        
        mgr.add_bar(b1);
        mgr.add_bar(b2);
        
        assert_eq!(mgr.bars_on_position(BarPosition::Top).len(), 1);
        assert_eq!(mgr.bars_on_position(BarPosition::Bottom).len(), 1);
        assert_eq!(mgr.total_modules(), 3);
    }

    #[test]
    fn test_disabled_modules_excluded() {
        let mut bar = Polybar::new("DP-1", BarPosition::Top);
        let mut m = PolybarModule::new("hidden", PolybarModuleKind::Custom("x".into()), "left");
        m.state = ModuleState::Disabled;
        bar.add_module(m);
        bar.add_module(PolybarModule::new("clock", PolybarModuleKind::Clock, "right"));
        
        assert_eq!(bar.enabled_modules().len(), 1);
        assert_eq!(bar.modules.len(), 2);
    }
}
