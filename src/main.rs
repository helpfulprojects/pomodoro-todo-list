#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use byte_unit::{Byte, UnitType};
use eframe::{
    egui::{self, debug_text, FontId, RichText, Rounding, TextBuffer},
    epaint,
};
use rusqlite::{Connection, Result};
use std::process::Command;
use std::{
    env, fs, io,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};

use egui_extras::{Column, TableBuilder};

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Video Editor For Devs",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<MyApp>::default())
        }),
    )
}

struct MyApp {
    test: i32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self { test: 32 }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.set_pixels_per_point(2.0);
            ui.label("Test");
        });
    }
}
