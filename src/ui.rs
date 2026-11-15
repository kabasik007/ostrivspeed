use std::time::{Duration, Instant};

use crate::{
    fingerprint::{self, Fingerprint},
    hotkeys::{Action, HotkeyService, PRESETS},
    process::{GameProcess, NativeProcessBackend, ProcessBackend},
    simulation_speed::SpeedController,
};
use eframe::egui::{self, Color32, RichText};

const BACK: Color32 = Color32::from_rgb(16, 19, 26);
const PANEL: Color32 = Color32::from_rgb(25, 30, 39);
const ACCENT: Color32 = Color32::from_rgb(76, 217, 181);
const MUTED: Color32 = Color32::from_rgb(151, 161, 176);

pub struct InspectorApp {
    process: Option<GameProcess>,
    fingerprint: Option<Fingerprint>,
    controller: Option<SpeedController>,
    hotkeys: HotkeyService,
    log: Vec<String>,
    armed: bool,
    process_filter: String,
    selected_speed: u32,
    last_scan: Instant,
    last_sample: Option<(Instant, f64)>,
    effective_speed: Option<f64>,
    stalled_samples: u8,
}

impl Default for InspectorApp {
    fn default() -> Self {
        let mut app = Self {
            process: None,
            fingerprint: None,
            controller: None,
            hotkeys: HotkeyService::new(),
            log: Vec::new(),
            armed: false,
            process_filter: String::new(),
            selected_speed: 1,
            last_scan: Instant::now(),
            last_sample: None,
            effective_speed: None,
            stalled_samples: 0,
        };
        app.refresh();
        app
    }
}

impl InspectorApp {
    fn log(&mut self, message: impl Into<String>) {
        self.log.push(message.into());
        if self.log.len() > 100 {
            self.log.remove(0);
        }
    }

    fn refresh(&mut self) {
        match NativeProcessBackend.find_game() {
            Ok(Some(game)) => {
                if self.process.as_ref().is_none_or(|old| old.pid != game.pid) {
                    self.detach();
                    self.fingerprint = match fingerprint::inspect(&game.path) {
                        Ok(fp) => Some(fp),
                        Err(error) => {
                            self.log(error);
                            None
                        }
                    };
                    self.log(format!("Ostriv PID {} found", game.pid));
                }
                self.process = Some(game);
            }
            Ok(None) => {
                if self.process.is_some() {
                    self.detach();
                    self.log("Ostriv process closed");
                }
                self.process = None;
                self.fingerprint = None;
            }
            Err(error) => self.log(format!("Process scan: {error}")),
        }
        self.last_scan = Instant::now();
    }

    fn attach(&mut self) {
        if !self.armed || self.controller.is_some() {
            return;
        }
        if let (Some(game), Some(fp)) = (&self.process, &self.fingerprint) {
            match SpeedController::attach(game, fp) {
                Ok(controller) => {
                    self.selected_speed = controller.read().unwrap_or(1.0) as u32;
                    self.controller = Some(controller);
                    self.last_sample = None;
                    self.log("Attached to verified game speed variable");
                }
                Err(error) => self.log(format!("Attach refused: {error}")),
            }
        }
    }

    fn detach(&mut self) {
        if let Some(mut controller) = self.controller.take() {
            match controller.set(1).and_then(|()| controller.restore()) {
                Ok(()) => self.log("Restored 1× and detached"),
                Err(error) => self.log(format!("Restore failed: {error}")),
            }
        }
        self.last_sample = None;
        self.effective_speed = None;
        self.stalled_samples = 0;
        self.selected_speed = 1;
    }

    fn apply_speed(&mut self, speed: u32) {
        if let Some(controller) = &mut self.controller {
            match controller.set(speed) {
                Ok(()) => {
                    self.selected_speed = speed;
                    self.last_sample = None;
                    self.effective_speed = None;
                    self.stalled_samples = 0;
                    self.log(format!("Native simulation speed set to {speed}×"));
                }
                Err(error) => self.log(format!("Speed change refused: {error}")),
            }
        }
    }

    fn handle_hotkeys(&mut self) {
        for action in self.hotkeys.poll() {
            if self.controller.is_none() {
                self.log("Shortcut ignored: attach to Ostriv first");
                continue;
            }
            let index = PRESETS
                .iter()
                .position(|&x| x == self.selected_speed)
                .unwrap_or(0);
            let speed = match action {
                Action::Preset(value) => value,
                Action::Next => PRESETS[(index + 1).min(PRESETS.len() - 1)],
                Action::Previous => PRESETS[index.saturating_sub(1)],
            };
            self.apply_speed(speed);
        }
    }

    fn sample_world(&mut self) {
        let Some(controller) = &mut self.controller else {
            return;
        };
        let (Ok(world), Ok(requested)) = (controller.world_time(), controller.read()) else {
            return;
        };
        let now = Instant::now();
        if let Some((previous_at, previous_world)) = self.last_sample {
            let elapsed = now.duration_since(previous_at).as_secs_f64();
            if elapsed >= 1.0 {
                let effective = (world - previous_world) / elapsed;
                self.effective_speed = Some(effective);
                self.stalled_samples = if requested > 12.0 && effective < 0.5 {
                    self.stalled_samples.saturating_add(1)
                } else {
                    0
                };
                self.last_sample = Some((now, world));
                if self.stalled_samples >= 2 {
                    let result = controller.set(1);
                    self.stalled_samples = 0;
                    self.selected_speed = 1;
                    match result {
                        Ok(()) => self.log("World time stalled; automatically restored 1×"),
                        Err(error) => self.log(format!("Auto restore failed: {error}")),
                    }
                }
            }
        } else {
            self.last_sample = Some((now, world));
        }
    }

    fn speed_card(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.label(RichText::new("КЕРУВАННЯ СИМУЛЯЦІЄЮ").color(MUTED).strong());
            ui.add_space(13.0);
            ui.label(
                RichText::new(format!("{}×", self.selected_speed))
                    .size(54.0)
                    .color(ACCENT)
                    .strong(),
            );
            ui.label(RichText::new("Календар · мешканці · будівництво · економіка").color(MUTED));
            ui.add_space(12.0);
            let mut index = PRESETS
                .iter()
                .position(|&x| x == self.selected_speed)
                .unwrap_or(0) as u32;
            if ui
                .add_enabled(
                    self.controller.is_some(),
                    egui::Slider::new(&mut index, 0..=6).show_value(false),
                )
                .changed()
            {
                self.apply_speed(PRESETS[index as usize]);
            }
            ui.horizontal_wrapped(|ui| {
                for speed in PRESETS {
                    let active = speed == self.selected_speed;
                    let label =
                        RichText::new(format!("{speed}×")).color(if active { BACK } else { MUTED });
                    let button = egui::Button::new(label).fill(if active { ACCENT } else { PANEL });
                    if ui.add_enabled(self.controller.is_some(), button).clicked() {
                        self.apply_speed(speed);
                    }
                }
            });
            ui.add_space(6.0);
            if ui
                .add_enabled(self.controller.is_some(), egui::Button::new("Повернути 1×"))
                .clicked()
            {
                self.apply_speed(1);
            }
            ui.label(
                RichText::new("60× вимкнено: під час вимірювання гра зупинилася.")
                    .color(MUTED)
                    .small(),
            );
        });
    }

    fn telemetry_card(&self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.label(RichText::new("ТЕЛЕМЕТРІЯ").color(MUTED).strong());
            ui.add_space(12.0);
            metric(
                ui,
                "Ціль у грі",
                self.controller.as_ref().and_then(|c| c.read().ok()),
            );
            metric(
                ui,
                "Поточна в грі",
                self.controller
                    .as_ref()
                    .and_then(|c| c.current_speed().ok()),
            );
            metric(ui, "Фактичний хід світу", self.effective_speed);
            ui.add_space(6.0);
            ui.label(
                RichText::new("Ігровий час за реальну секунду.")
                    .color(MUTED)
                    .small(),
            );
        });
    }

    fn process_card(&mut self, ui: &mut egui::Ui) {
        let (mut refresh, mut attach, mut detach) = (false, false, false);
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("ГРА І ПІДКЛЮЧЕННЯ").color(MUTED).strong());
                if ui.button("Оновити").clicked() {
                    refresh = true;
                }
            });
            ui.add_space(8.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.process_filter)
                    .hint_text("Пошук процесу Ostriv"),
            );
            if let Some(game) = &self.process {
                if "ostriv.exe".contains(&self.process_filter.to_lowercase()) {
                    ui.label(
                        RichText::new(format!("●  ostriv.exe   ·   PID {}", game.pid))
                            .color(ACCENT)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(game.path.display().to_string())
                            .color(MUTED)
                            .small(),
                    );
                } else {
                    ui.label("Немає збігів");
                }
            } else {
                ui.label(RichText::new("Ostriv не запущено").color(MUTED));
            }
            let recognized = self.fingerprint.as_ref().is_some_and(|fp| fp.recognized);
            ui.label(
                RichText::new(if recognized {
                    "Збірка перевірена за SHA-256"
                } else {
                    "Невідома збірка: запис заблоковано"
                })
                .color(if recognized { ACCENT } else { Color32::YELLOW }),
            );
            if let Some(fp) = &self.fingerprint {
                ui.label(
                    RichText::new(format!(
                        "{} · SHA-256 {}…",
                        fp.architecture,
                        &fp.sha256[..12]
                    ))
                    .color(MUTED)
                    .small(),
                );
            }
            ui.add_space(4.0);
            ui.checkbox(&mut self.armed, "Дозволити керування для цієї сесії");
            if !self.armed && self.controller.is_some() {
                detach = true;
            }
            if self.controller.is_some() {
                if ui.button("Від'єднати та повернути 1×").clicked() {
                    detach = true;
                }
            } else if ui
                .add_enabled(self.armed && recognized, egui::Button::new("Під'єднатися"))
                .clicked()
            {
                attach = true;
            }
        });
        if refresh {
            self.refresh();
        }
        if detach {
            self.detach();
        }
        if attach {
            self.attach();
        }
    }

    fn hotkeys_card(&self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.label(RichText::new("ГАРЯЧІ КЛАВІШІ").color(MUTED).strong());
            ui.add_space(10.0);
            ui.label("Ctrl + Alt + 0    :    1× / скидання");
            ui.label("Ctrl + Alt + 1…7    :    1×, 5×, 12×, 15×, 20×, 30×, 50×");
            ui.label("Ctrl + Alt + Up / Down    :    наступна / попередня");
            ui.add_space(8.0);
            ui.label(RichText::new(&self.hotkeys.status).color(ACCENT).small());
            ui.label(
                RichText::new("Комбінації діють лише після під'єднання.")
                    .color(MUTED)
                    .small(),
            );
        });
    }

    fn diagnostics_card(&self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.label(RichText::new("ДІАГНОСТИКА").color(MUTED).strong());
            egui::ScrollArea::vertical()
                .max_height(125.0)
                .show(ui, |ui| {
                    for line in self.log.iter().rev().take(15) {
                        ui.label(RichText::new(line).color(MUTED).small());
                    }
                });
        });
    }
}

impl eframe::App for InspectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_scan.elapsed() >= Duration::from_secs(3) {
            self.refresh();
        }
        self.handle_hotkeys();
        self.sample_world();
        ctx.request_repaint_after(Duration::from_millis(200));
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(BACK).inner_margin(24)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("OSTRIV").size(25.0).strong());
                ui.label(RichText::new("SPEED").size(25.0).color(ACCENT).strong());
                ui.add_space(15.0);
                ui.label(RichText::new("Нативне прискорення симуляції").color(MUTED));
            });
            ui.add_space(20.0);
            ui.columns(2, |columns| {
                self.speed_card(&mut columns[0]);
                self.telemetry_card(&mut columns[1]);
            });
            ui.add_space(12.0);
            ui.columns(2, |columns| {
                self.process_card(&mut columns[0]);
                self.hotkeys_card(&mut columns[1]);
            });
            ui.add_space(12.0);
            self.diagnostics_card(ui);
            ui.add_space(7.0);
            ui.label(RichText::new("Керує лише підтвердженою змінною Ostriv. Таймери Windows, звук і камера не перехоплюються.").color(MUTED).small());
        });
    }
}

fn card(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();
    egui::Frame::new()
        .fill(PANEL)
        .corner_radius(12)
        .inner_margin(17)
        .show(ui, |ui| {
            ui.set_min_width((width - 34.0).max(0.0));
            content(ui);
        });
}

fn metric(ui: &mut egui::Ui, label: &str, value: Option<f64>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(MUTED));
        ui.label(
            RichText::new(value.map_or("—".into(), |v| format!("{v:.1}×")))
                .color(ACCENT)
                .strong(),
        );
    });
}
