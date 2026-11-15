use std::time::{Duration, Instant};

use crate::{
    fingerprint::{self, Fingerprint},
    hotkeys::{self, Action, Binding, HotkeyService, PRESETS},
    process::{GameProcess, NativeProcessBackend, ProcessBackend},
    simulation_speed::SpeedController,
};
use eframe::egui::{self, Color32, RichText};

const BACK: Color32 = Color32::from_rgb(29, 35, 45);
const PANEL: Color32 = Color32::from_rgb(43, 52, 65);
const ACCENT: Color32 = Color32::from_rgb(76, 217, 181);
const MUTED: Color32 = Color32::from_rgb(151, 161, 176);

pub struct InspectorApp {
    process: Option<GameProcess>,
    fingerprint: Option<Fingerprint>,
    controller: Option<SpeedController>,
    hotkeys: HotkeyService,
    bindings: Vec<Binding>,
    capture_index: Option<usize>,
    log: Vec<String>,
    armed: bool,
    trial_armed: bool,
    trial: Option<Trial>,
    process_filter: String,
    selected_speed: u32,
    last_scan: Instant,
    last_sample: Option<(Instant, f64)>,
    effective_speed: Option<f64>,
    stalled_samples: u8,
}

struct Trial {
    speed: u32,
    started: Instant,
    world_start: f64,
    cpu_start: f64,
}

impl Default for InspectorApp {
    fn default() -> Self {
        let bindings = hotkeys::load();
        let mut app = Self {
            process: None,
            fingerprint: None,
            controller: None,
            hotkeys: HotkeyService::inactive(),
            bindings,
            capture_index: None,
            log: Vec::new(),
            armed: false,
            trial_armed: false,
            trial: None,
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
                    self.hotkeys = HotkeyService::new(&self.bindings);
                    self.last_sample = None;
                    self.log("Attached to verified game speed variable");
                }
                Err(error) => self.log(format!("Attach refused: {error}")),
            }
        }
    }

    fn detach(&mut self) {
        self.trial = None;
        self.trial_armed = false;
        self.hotkeys.stop();
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
        self.trial = None;
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

    fn start_trial(&mut self, speed: u32) {
        if !self.trial_armed || self.trial.is_some() {
            return;
        }
        let Some(controller) = &mut self.controller else {
            return;
        };
        let (Ok(world_start), Ok(cpu_start)) = (controller.world_time(), controller.cpu_seconds())
        else {
            self.log("Trial refused: telemetry unavailable");
            return;
        };
        match controller.set_experimental(speed) {
            Ok(()) => {
                self.trial = Some(Trial {
                    speed,
                    started: Instant::now(),
                    world_start,
                    cpu_start,
                });
                self.selected_speed = speed;
                self.last_sample = None;
                self.log(format!(
                    "Experimental {speed}× started; automatic 1× reset in 2 seconds"
                ));
            }
            Err(error) => self.log(format!("Trial refused: {error}")),
        }
    }

    fn finish_trial(&mut self, reason: &str) {
        let Some(trial) = self.trial.take() else {
            return;
        };
        let Some(controller) = &mut self.controller else {
            return;
        };
        let elapsed = trial.started.elapsed().as_secs_f64();
        let world_delta = controller.world_time().unwrap_or(f64::NAN) - trial.world_start;
        let cpu_delta = controller.cpu_seconds().unwrap_or(f64::NAN) - trial.cpu_start;
        let result = controller.set(1);
        self.selected_speed = if result.is_ok() {
            1
        } else {
            controller.read().unwrap_or(trial.speed as f64) as u32
        };
        self.last_sample = None;
        self.trial_armed = false;
        self.log(format!(
            "Trial {}× {reason}: {:.2}× effective over {:.2}s, {:.2} CPU seconds",
            trial.speed,
            world_delta / elapsed,
            elapsed,
            cpu_delta
        ));
        if let Err(error) = result {
            self.log(format!("Trial reset failed: {error}"));
        }
    }

    fn capture_hotkey(&mut self, ctx: &egui::Context) {
        let Some(index) = self.capture_index else {
            return;
        };
        let event = ctx.input(|i| {
            i.events.iter().find_map(|event| {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } = event
                {
                    Some((*key, *modifiers))
                } else {
                    None
                }
            })
        });
        let Some((key, modifiers)) = event else {
            return;
        };
        if key == egui::Key::Escape {
            self.capture_index = None;
            if self.controller.is_some() {
                self.hotkeys = HotkeyService::new(&self.bindings);
            }
            self.log("Hotkey capture cancelled");
            return;
        }
        let Some((vk, mods)) = hotkeys::capture(key, modifiers) else {
            self.log("Use F1–F24, or a modified letter, digit, Up or Down");
            return;
        };
        if self
            .bindings
            .iter()
            .enumerate()
            .any(|(i, b)| i != index && b.key == vk && b.modifiers == mods)
        {
            self.log("This shortcut is already assigned");
            return;
        }
        self.bindings[index].key = vk;
        self.bindings[index].modifiers = mods;
        self.capture_index = None;
        if self.controller.is_some() {
            self.hotkeys = HotkeyService::new(&self.bindings);
        }
        if let Err(error) = hotkeys::save(&self.bindings) {
            self.log(format!("Hotkey was applied but not saved: {error}"));
        } else {
            self.log(format!(
                "Hotkey saved: {}",
                hotkeys::label(self.bindings[index])
            ));
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
        if self
            .trial
            .as_ref()
            .is_some_and(|trial| trial.started.elapsed() >= Duration::from_secs(2))
        {
            self.finish_trial("time limit reached");
            return;
        }
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
                if self.trial.is_some() && effective < 0.5 {
                    self.finish_trial("world time stalled");
                    return;
                }
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
            ui.separator();
            ui.label(RichText::new("КОРОТКИЙ ЕКСПЕРИМЕНТ").color(MUTED).strong());
            ui.checkbox(&mut self.trial_armed, "Дозволити тест на 2 секунди");
            ui.horizontal(|ui| {
                for speed in [60, 70, 100] {
                    if ui
                        .add_enabled(
                            self.controller.is_some() && self.trial_armed && self.trial.is_none(),
                            egui::Button::new(format!("Тест {speed}×")),
                        )
                        .clicked()
                    {
                        self.start_trial(speed);
                    }
                }
            });
            if let Some(trial) = &self.trial {
                ui.label(
                    RichText::new(format!(
                        "{}×: {:.1} с до скидання",
                        trial.speed,
                        (2.0 - trial.started.elapsed().as_secs_f64()).max(0.0)
                    ))
                    .color(Color32::YELLOW),
                );
            } else {
                ui.label(
                    RichText::new("60× раніше зупинило гру; 70× і 100× не перевірені.")
                        .color(MUTED)
                        .small(),
                );
            }
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

    fn hotkeys_card(&mut self, ui: &mut egui::Ui) {
        let mut capture_request = None;
        card(ui, |ui| {
            ui.label(RichText::new("ГАРЯЧІ КЛАВІШІ").color(MUTED).strong());
            ui.label(
                RichText::new("Натисніть на комбінацію, потім введіть нову; Esc скасовує.")
                    .color(MUTED)
                    .small(),
            );
            egui::ScrollArea::vertical()
                .max_height(140.0)
                .show(ui, |ui| {
                    for (index, binding) in self.bindings.iter().enumerate() {
                        let action = match index {
                            0 => "Скинути 1×".to_string(),
                            1..=7 => format!("Швидкість {}×", PRESETS[index - 1]),
                            8 => "Наступна".to_string(),
                            _ => "Попередня".to_string(),
                        };
                        ui.horizontal(|ui| {
                            ui.label(action);
                            let label = if self.capture_index == Some(index) {
                                "Натисніть клавіші…".into()
                            } else {
                                hotkeys::label(*binding)
                            };
                            if ui.button(label).clicked() {
                                capture_request = Some(index);
                            }
                        });
                    }
                });
            ui.label(RichText::new(&self.hotkeys.status).color(ACCENT).small());
            ui.label(
                RichText::new("Глобальні клавіші діють лише після під'єднання.")
                    .color(MUTED)
                    .small(),
            );
        });
        if let Some(index) = capture_request {
            self.hotkeys.stop();
            self.hotkeys.status = "Hotkeys paused for capture".into();
            self.capture_index = Some(index);
        }
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
        self.capture_hotkey(ctx);
        self.handle_hotkeys();
        self.sample_world();
        ctx.request_repaint_after(Duration::from_millis(200));
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(BACK).inner_margin(24)).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
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
