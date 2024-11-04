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

fn setup_database() -> Result<Connection> {
    let conn = Connection::open("tasks.db")?;
    //conn.execute("DROP TABLE IF EXISTS tasks", ())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            id    INTEGER PRIMARY KEY,
            name  TEXT NOT NULL,
            done INTEGER,
            estimate INTEGER,
            locked INTEGER,
            just_created INTEGER
        )",
        (),
    )?;
    Ok(conn)
}

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
    id: i32,
    name: String,
    done: bool,
    estimate: i32,
    locked: bool,
    just_created: bool,
}

struct MyApp {
    conn: Connection,
    show_new_task_input: bool,
    new_task_name: String,
    tasks: Vec<Task>,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut self_setup = Self {
            conn: setup_database().unwrap(),
            show_new_task_input: false,
            new_task_name: "".to_string(),
            tasks: vec![],
        };
        self_setup.tasks = get_tasks(&self_setup.conn);
        self_setup
    }
}

fn get_tasks(conn: &Connection) -> Vec<Task> {
    let mut tasks: Vec<Task> = vec![];
    let mut stmt = conn.prepare("SELECT * FROM tasks").unwrap();
    let tasks_iter = stmt
        .query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                done: row.get(2)?,
                estimate: row.get(3)?,
                locked: row.get(4)?,
                just_created: row.get(5)?,
            })
        })
        .unwrap();
    for task in tasks_iter {
        tasks.push(task.unwrap());
    }
    tasks
}

fn set_task_status(conn: &mut Connection, done: bool, id: i32) {
    let tx = conn.transaction().unwrap();
    tx.execute("UPDATE tasks SET done = ?1 where id = ?2", (done, id))
        .unwrap();
    tx.commit().unwrap();
}

fn set_task_locked(conn: &mut Connection, locked: bool, id: i32) {
    let tx = conn.transaction().unwrap();
    tx.execute("UPDATE tasks SET locked = ?1 where id = ?2", (locked, id))
        .unwrap();
    tx.commit().unwrap();
}

fn set_task_just_created(conn: &mut Connection, just_created: bool, id: i32) {
    let tx = conn.transaction().unwrap();
    tx.execute(
        "UPDATE tasks SET just_created = ?1 where id = ?2",
        (just_created, id),
    )
    .unwrap();
    tx.commit().unwrap();
}
fn set_task_name(conn: &mut Connection, name: String, id: i32) {
    let tx = conn.transaction().unwrap();
    tx.execute("UPDATE tasks SET name = ?1 where id = ?2", (name, id))
        .unwrap();
    tx.commit().unwrap();
}

fn delete_task(conn: &mut Connection, id: i32) {
    let tx = conn.transaction().unwrap();
    tx.execute("DELETE from tasks where id = ?1", [id]).unwrap();
    tx.commit().unwrap();
}

fn create_task(conn: &mut Connection, task: Task) {
    let tx = conn.transaction().unwrap();
    tx.execute(
        "INSERT INTO tasks (name, done, estimate, locked, just_created) VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            task.name,
            task.done,
            task.estimate,
            task.locked,
            task.just_created,
        ),
    )
    .unwrap();
    tx.commit().unwrap();
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.set_pixels_per_point(2.0);
            let mut update_ui = false;
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
                            for task in self.tasks.iter_mut() {
                                if task.locked {
                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut task.done, "").changed() {
                                            set_task_status(&mut self.conn, task.done, task.id);
                                            update_ui = true;
                                        };
                                        if ui.label(task.name.clone()).double_clicked() {
                                            set_task_locked(&mut self.conn, false, task.id);
                                            update_ui = true;
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
                                        set_task_locked(&mut self.conn, true, task.id);
                                        set_task_just_created(&mut self.conn, true, task.id);
                                        set_task_name(&mut self.conn, task.name.clone(), task.id);
                                        if task.name.is_empty() {
                                            delete_task(&mut self.conn, task.id);
                                        }
                                        update_ui = true;
                                        if task.just_created {
                                            self.show_new_task_input = false;
                                        }
                                    }
                                    if task.just_created {
                                        //response.request_focus();
                                    }
                                }
                            }
                            if !self.show_new_task_input {
                                if ui.button("+ Add Task").clicked() {
                                    create_task(
                                        &mut self.conn,
                                        Task {
                                            id: 0,
                                            name: self.new_task_name.clone(),
                                            done: false,
                                            locked: false,
                                            estimate: 0,
                                            just_created: true,
                                        },
                                    );
                                    self.new_task_name = "".to_string();
                                    self.show_new_task_input = true;
                                    update_ui = true;
                                }
                            }
                        });
                        row.col(|ui| {
                            //ui.label("Add Task");
                        });
                    });
                });
            if update_ui {
                self.tasks = get_tasks(&self.conn);
            }
        });
    }
}
