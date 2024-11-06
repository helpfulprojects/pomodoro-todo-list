#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use eframe::egui::{self, Button, Color32, ImageButton, RichText};
use rodio::{source::Source, Decoder, OutputStream};
use rusqlite::{Connection, Result};
use std::io::BufReader;
use std::{cmp::max, fs::File};
use time::{Date, Duration, Month, OffsetDateTime, Time, UtcOffset};
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
    conn.execute(
        "CREATE TABLE IF NOT EXISTS timers (
            id    INTEGER PRIMARY KEY,
            is_pomodoro  INTEGER,
            start INTEGER,
            duration INTEGER,
            task INTEGER,
            FOREIGN KEY(task) REFERENCES tasks(id)
        )",
        (),
    )?;
    Ok(conn)
}

const FOCUS_DURATION: i32 = 1;
const SHORT_BREAK_DURATION: i32 = 1;
const LONG_BREAK_DURATION: i32 = 2;
const DEFAULT_WINDOW_TITLE: &str = "Pomodoro To Do List";

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        DEFAULT_WINDOW_TITLE,
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

struct Timer {
    id: i32,
    is_pomodoro: bool,
    start: OffsetDateTime,
    duration: i32,
    task: Option<i32>,
}

struct MyApp {
    conn: Connection,
    show_new_task_input: bool,
    new_task_name: String,
    tasks: Vec<Task>,
    played_notification: bool,
    pomodoros_estimate: i32,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut self_setup = Self {
            conn: setup_database().unwrap(),
            show_new_task_input: false,
            new_task_name: "".to_string(),
            tasks: vec![],
            played_notification: false,
            pomodoros_estimate: 0,
        };
        self_setup.tasks = get_tasks(&self_setup.conn);
        add_pomodoros(
            &mut self_setup.conn,
            20,
            OffsetDateTime::new_in_offset(
                Date::from_calendar_date(2024, Month::November, 5).unwrap(),
                Time::from_hms(12, 59, 59).unwrap(),
                UtcOffset::from_hms(2, 0, 0).unwrap(),
            ),
        );
        add_pomodoros(
            &mut self_setup.conn,
            8,
            OffsetDateTime::new_in_offset(
                Date::from_calendar_date(2024, Month::November, 4).unwrap(),
                Time::from_hms(12, 59, 59).unwrap(),
                UtcOffset::from_hms(2, 0, 0).unwrap(),
            ),
        );
        add_pomodoros(
            &mut self_setup.conn,
            7,
            OffsetDateTime::new_in_offset(
                Date::from_calendar_date(2024, Month::November, 3).unwrap(),
                Time::from_hms(12, 59, 59).unwrap(),
                UtcOffset::from_hms(2, 0, 0).unwrap(),
            ),
        );
        add_pomodoros(
            &mut self_setup.conn,
            6,
            OffsetDateTime::new_in_offset(
                Date::from_calendar_date(2024, Month::November, 2).unwrap(),
                Time::from_hms(12, 59, 59).unwrap(),
                UtcOffset::from_hms(2, 0, 0).unwrap(),
            ),
        );
        add_pomodoros(
            &mut self_setup.conn,
            6,
            OffsetDateTime::new_in_offset(
                Date::from_calendar_date(2024, Month::November, 1).unwrap(),
                Time::from_hms(12, 59, 59).unwrap(),
                UtcOffset::from_hms(2, 0, 0).unwrap(),
            ),
        );
        self_setup.pomodoros_estimate = get_pomodoros_median(&mut self_setup.conn);

        self_setup
    }
}

fn play_notificaiton() {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let file = BufReader::new(File::open("assets/notification.mp3").unwrap());
    let source = Decoder::new(file).unwrap();
    stream_handle.play_raw(source.convert_samples()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1000));
}

fn get_tasks(conn: &Connection) -> Vec<Task> {
    let mut tasks: Vec<Task> = vec![];
    let mut stmt = conn.prepare("SELECT * FROM tasks where done = 0").unwrap();
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

fn get_running_timers(conn: &mut Connection) -> Vec<Timer> {
    let mut timers: Vec<Timer> = vec![];
    let mut stmt = conn
        .prepare("SELECT * FROM timers where task is NULL")
        .unwrap();
    let timers_iter = stmt
        .query_map([], |row| {
            Ok(Timer {
                id: row.get(0)?,
                is_pomodoro: row.get(1)?,
                start: row.get(2)?,
                duration: row.get(3)?,
                task: row.get(4)?,
            })
        })
        .unwrap();
    for timer in timers_iter {
        timers.push(timer.unwrap());
    }
    timers
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

fn set_task_estimate(conn: &mut Connection, estimate: i32, id: i32) {
    let tx = conn.transaction().unwrap();
    tx.execute(
        "UPDATE tasks SET estimate = ?1 where id = ?2",
        (estimate, id),
    )
    .unwrap();
    tx.commit().unwrap();
}

fn delete_task(conn: &mut Connection, id: i32) {
    let tx = conn.transaction().unwrap();
    tx.execute("DELETE from tasks where id = ?1", [id]).unwrap();
    tx.commit().unwrap();
}

fn mean(numbers: &Vec<i32>) -> f32 {
    let sum: i32 = numbers.iter().sum();

    sum as f32 / numbers.len() as f32
}

fn median(numbers: &mut Vec<i32>) -> i32 {
    numbers.sort();

    let mid = numbers.len() / 2;
    if numbers.len() % 2 == 0 {
        mean(&vec![numbers[mid - 1], numbers[mid]]) as i32
    } else {
        numbers[mid]
    }
}

fn get_pomodoros_median(conn: &mut Connection) -> i32 {
    let mut pomodoros: Vec<i32> = vec![];
    let mut stmt = conn
        .prepare("select count(start) from timers where start >= date('now','-30 days') and start < date('now') and time(start) >= time('now') group by date(start)")
        .unwrap();
    let pomodoros_iter = stmt.query_map([], |row| Ok(row.get(0)?)).unwrap();
    for pomodoro_count in pomodoros_iter {
        pomodoros.push(pomodoro_count.unwrap());
    }
    if pomodoros.len() == 0 {
        0
    } else {
        median(&mut pomodoros)
    }
}

fn create_timer(conn: &mut Connection, timer: Timer) {
    let tx = conn.transaction().unwrap();
    match timer.task {
        Some(task) => {
            tx.execute(
                "INSERT INTO timers (is_pomodoro, start, duration, task) VALUES (?1, ?2, ?3, ?4)",
                (timer.is_pomodoro, timer.start, timer.duration, task),
            )
            .unwrap();
        }
        None => {
            tx.execute(
                "INSERT INTO timers (is_pomodoro, start, duration) VALUES (?1, ?2, ?3)",
                (timer.is_pomodoro, timer.start, timer.duration),
            )
            .unwrap();
        }
    }

    tx.commit().unwrap();
}

fn delete_pomodoros_without_task(conn: &mut Connection) {
    let tx = conn.transaction().unwrap();
    tx.execute("DELETE from timers where task is NULL", [])
        .unwrap();
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

fn add_pomodoros(conn: &mut Connection, amount: i32, date: OffsetDateTime) {
    for n in 1..=amount {
        create_timer(
            conn,
            Timer {
                id: 0,
                is_pomodoro: true,
                start: date,
                duration: 25,
                task: Some(1),
            },
        );
    }
}

fn get_task_pomodoros(conn: &mut Connection, task_id: i32) -> usize {
    let mut timers: Vec<Timer> = vec![];
    let mut stmt = conn
        .prepare("SELECT * FROM timers where task = :id")
        .unwrap();
    let timers_iter = stmt
        .query_map(&[(":id", &task_id)], |row| {
            Ok(Timer {
                id: row.get(0)?,
                is_pomodoro: row.get(1)?,
                start: row.get(2)?,
                duration: row.get(3)?,
                task: row.get(4)?,
            })
        })
        .unwrap();
    for timer in timers_iter {
        timers.push(timer.unwrap());
    }
    timers.len()
}

fn is_timer_over(timer: &Timer) -> bool {
    let start = timer.start;
    let duration = timer.duration;
    let now = OffsetDateTime::now_local().unwrap();
    let end = start
        .checked_add(Duration::minutes(duration.into()))
        .unwrap();
    let difference = end - now;
    let seconds = difference.whole_seconds() % 60;
    let minutes = (difference.whole_seconds() / 60) % 60;
    (seconds < 0 || minutes < 0) && timer.is_pomodoro
}

fn update_timer_task(conn: &mut Connection, timer_id: i32, task_id: i32) {
    let tx = conn.transaction().unwrap();
    tx.execute(
        "UPDATE timers SET task = ?1 where id = ?2",
        (task_id, timer_id),
    )
    .unwrap();
    tx.commit().unwrap();
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.set_pixels_per_point(2.0);
            let mut update_ui = false;
            let timers = get_running_timers(&mut self.conn);
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
                        if timers.len() > 0 && is_timer_over(&timers[0]) {
                            if ui
                                .button("+")
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                update_timer_task(&mut self.conn, timers[0].id, task.id);
                                update_ui = true;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                                    DEFAULT_WINDOW_TITLE.to_string(),
                                ));
                            }
                        }
                        let pomodoros = get_task_pomodoros(&mut self.conn, task.id);
                        for n in 1..=pomodoros {
                            ui.image(egui::include_image!("../assets/pomodoro.png"));
                        }
                        if task.estimate > pomodoros.try_into().unwrap() {
                            for n in 1..=task.estimate - pomodoros as i32 {
                                if ui
                                    .add(ImageButton::frame(
                                        ImageButton::new(egui::include_image!(
                                            "../assets/estimation.png"
                                        )),
                                        false,
                                    ))
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    set_task_estimate(&mut self.conn, task.estimate - 1, task.id);
                                    update_ui = true;
                                }
                            }
                        }

                        if ui
                            .add(ImageButton::frame(
                                ImageButton::new(egui::include_image!(
                                    "../assets/add_estimation.png"
                                )),
                                false,
                            ))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            let new_estimation =
                                max(pomodoros, task.estimate.try_into().unwrap()) + 1;
                            set_task_estimate(
                                &mut self.conn,
                                new_estimation.try_into().unwrap(),
                                task.id,
                            );
                            update_ui = true;
                        }
                    });
                } else {
                    let response = ui
                        .add(egui::TextEdit::singleline(&mut task.name).hint_text("Task name..."));
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
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
                        response.request_focus();
                    }
                }
            }
            if !self.show_new_task_input {
                if ui
                    .add(egui::Button::frame(egui::Button::new("+ Add Task"), false))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
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
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.horizontal(|ui| {
                    ui.scope(|ui| {
                        ui.style_mut().visuals.widgets.hovered.weak_bg_fill =
                            Color32::from_hex("#A80000").unwrap();
                        let focus_button: Button;
                        if timers.len() > 0 && timers[0].is_pomodoro {
                            focus_button = Button::fill(
                                Button::new(
                                    RichText::new(format!("Focus x{}", self.pomodoros_estimate))
                                        .color(Color32::from_hex("#FFF9F0").unwrap()),
                                ),
                                Color32::from_hex("#A80000").unwrap(),
                            );
                        } else {
                            focus_button =
                                Button::new(format!("Focus x{}", self.pomodoros_estimate));
                        }
                        if ui
                            .add(focus_button)
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            delete_pomodoros_without_task(&mut self.conn);
                            create_timer(
                                &mut self.conn,
                                Timer {
                                    id: 0,
                                    is_pomodoro: true,
                                    start: OffsetDateTime::now_local().unwrap(),
                                    duration: FOCUS_DURATION,
                                    task: Some(0),
                                },
                            );
                            ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                                DEFAULT_WINDOW_TITLE.to_string(),
                            ));
                            self.played_notification = false;
                        }
                    });
                    ui.scope(|ui| {
                        ui.style_mut().visuals.widgets.hovered.weak_bg_fill =
                            Color32::from_hex("#005C00").unwrap();
                        let short_break_button: Button;
                        if timers.len() > 0
                            && !timers[0].is_pomodoro
                            && timers[0].duration == SHORT_BREAK_DURATION
                        {
                            short_break_button = Button::fill(
                                Button::new(
                                    RichText::new("Short Break")
                                        .color(Color32::from_hex("#FFF9F0").unwrap()),
                                ),
                                Color32::from_hex("#005C00").unwrap(),
                            );
                        } else {
                            short_break_button = Button::new("Short Break");
                        }
                        if ui
                            .add(short_break_button)
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            delete_pomodoros_without_task(&mut self.conn);
                            create_timer(
                                &mut self.conn,
                                Timer {
                                    id: 0,
                                    is_pomodoro: false,
                                    start: OffsetDateTime::now_local().unwrap(),
                                    duration: SHORT_BREAK_DURATION,
                                    task: Some(0),
                                },
                            );
                            ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                                DEFAULT_WINDOW_TITLE.to_string(),
                            ));
                            self.played_notification = false;
                        }
                    });
                    ui.scope(|ui| {
                        ui.style_mut().visuals.widgets.hovered.weak_bg_fill =
                            Color32::from_hex("#1F1FFF").unwrap();
                        let long_break_button: Button;
                        if timers.len() > 0
                            && !timers[0].is_pomodoro
                            && timers[0].duration == LONG_BREAK_DURATION
                        {
                            long_break_button = Button::fill(
                                Button::new(
                                    RichText::new("Long Break")
                                        .color(Color32::from_hex("#FFF9F0").unwrap()),
                                ),
                                Color32::from_hex("#1F1FFF").unwrap(),
                            );
                        } else {
                            long_break_button = Button::new("Long Break");
                        }
                        if ui
                            .add(long_break_button)
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            delete_pomodoros_without_task(&mut self.conn);
                            create_timer(
                                &mut self.conn,
                                Timer {
                                    id: 0,
                                    is_pomodoro: false,
                                    start: OffsetDateTime::now_local().unwrap(),
                                    duration: LONG_BREAK_DURATION,
                                    task: Some(0),
                                },
                            );
                            ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                                DEFAULT_WINDOW_TITLE.to_string(),
                            ));
                            self.played_notification = false;
                        }
                    });
                    if timers.len() > 0 {
                        let timer = &timers[0];
                        let start = timer.start;
                        let duration = timer.duration;
                        let now = OffsetDateTime::now_local().unwrap();
                        let end = start
                            .checked_add(Duration::minutes(duration.into()))
                            .unwrap();
                        let difference = end - now;
                        let seconds = difference.whole_seconds() % 60;
                        let minutes = (difference.whole_seconds() / 60) % 60;
                        if seconds < 0 || minutes < 0 {
                            if timer.is_pomodoro {
                                ui.label("Done! Add point to task.");
                            } else {
                                delete_pomodoros_without_task(&mut self.conn);
                            }
                            if !self.played_notification {
                                play_notificaiton();
                                self.played_notification = true;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                                    DEFAULT_WINDOW_TITLE.to_string(),
                                ));
                            }
                        } else {
                            ui.label(format!("{:0>2}:{:0>2}", minutes, seconds));
                            if timer.is_pomodoro {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                                    "{:0>2}:{:0>2} Focus",
                                    minutes, seconds
                                )));
                            } else {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                                    "{:0>2}:{:0>2}",
                                    minutes, seconds
                                )));
                            }
                        }

                        if ui
                            .add(egui::Button::frame(egui::Button::new("x"), false))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            delete_pomodoros_without_task(&mut self.conn);
                            ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                                DEFAULT_WINDOW_TITLE.to_string(),
                            ));
                        }
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(300));
                    }
                });
            });

            if update_ui {
                self.tasks = get_tasks(&self.conn);
                self.pomodoros_estimate = get_pomodoros_median(&mut self.conn);
            }
        });
    }
}
