// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

mod app;
mod logic;
mod models;
mod mvu;
mod ui;
mod utils;

fn main() -> eframe::Result<()> {
    app::run()
}
