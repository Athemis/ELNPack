// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Binary entry point that boots the egui application.

mod app;
mod logic;
mod models;
mod mvu;
mod ui;
mod utils;

/// Launch the ELNPack desktop application.
fn main() -> eframe::Result<()> {
    app::run()
}
