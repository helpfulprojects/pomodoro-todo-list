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
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Pomodoro To Do List",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<MyApp>::default())
        }),
    )
}

struct Task {
    done: bool,
    name: String,
    locked: bool,
    created: bool,
}

struct MyApp {
    show_new_task_input: bool,
    new_task_name: String,
    tasks: Vec<Task>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            show_new_task_input: false,
            new_task_name: "".to_string(),
            tasks: vec![],
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.set_pixels_per_point(2.0);
            TableBuilder::new(ui)
                .column(Column::remainder())
                .column(Column::remainder())
                .header(10.0, |mut header| {
                    header.col(|ui| {
                        //ui.heading("Task");
                    });
                    header.col(|ui| {
                        //ui.heading("Size");
                    });
                })
                .body(|mut body| {
                    body.row(10.0, |mut row| {
                        row.col(|ui| {
                            for (index, task) in self.tasks.iter_mut().enumerate() {
                                if task.locked {
                                    ui.horizontal(|ui| {
                                        ui.checkbox(&mut task.done, "");
                                        if ui.label(task.name.clone()).double_clicked() {
                                            task.locked = false;
                                        }
                                    });
                                } else {
                                    let response = ui.add(
                                        egui::TextEdit::singleline(&mut task.name)
                                            .hint_text("Task name..."),
                                    );
                                    if response.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    {
                                        task.locked = true;
                                        task.created = true;
                                        // if task.name.is_empty() {
                                        //     self.tasks.remove(index);
                                        // }
                                    }
                                    if !task.created {
                                        response.request_focus();
                                        self.show_new_task_input = false;
                                    }
                                }
                            }
                            if !self.show_new_task_input {
                                if ui.button("+ Add Task").clicked() {
                                    self.tasks.push(Task {
                                        done: false,
                                        name: self.new_task_name.clone(),
                                        locked: false,
                                        created: false,
                                    });
                                    self.new_task_name = "".to_string();
                                    self.show_new_task_input = true;
                                }
                            }
                        });
                        row.col(|ui| {
                            //ui.label("Add Task");
                        });
                    });
                });
        });
    }
}
