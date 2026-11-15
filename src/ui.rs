use crate::{
    fingerprint::{self, Fingerprint},
    process::{GameProcess, NativeProcessBackend, ProcessBackend},
    simulation_speed::SpeedController,
};

#[derive(Default)]
pub struct InspectorApp {
    process: Option<GameProcess>,
    fingerprint: Option<Fingerprint>,
    log: Vec<String>,
    initialized: bool,
    experimental: bool,
    controller: Option<SpeedController>,
    last_sample: Option<(std::time::Instant, f64)>,
    effective_speed: Option<f64>,
    stalled_samples: u8,
}

impl InspectorApp {
    fn refresh(&mut self) {
        self.controller = None;
        self.last_sample = None;
        self.effective_speed = None;
        self.stalled_samples = 0;
        match NativeProcessBackend.find_game() {
            Ok(Some(process)) => {
                self.fingerprint = match fingerprint::inspect(&process.path) {
                    Ok(value) => Some(value),
                    Err(error) => {
                        self.log.push(error);
                        None
                    }
                };
                self.log.push(format!("Found Ostriv PID {}", process.pid));
                self.process = Some(process);
            }
            Ok(None) => {
                self.process = None;
                self.fingerprint = None;
                self.log.push("Ostriv process not found".into());
            }
            Err(error) => self.log.push(error),
        }
    }
}

impl eframe::App for InspectorApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if !self.initialized {
            self.refresh();
            self.initialized = true;
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
        if let Some(controller) = &mut self.controller
            && let (Ok(world), Ok(requested)) = (controller.world_time(), controller.read())
        {
            let now = std::time::Instant::now();
            if let Some((previous_at, previous_world)) = self.last_sample
                && now.duration_since(previous_at).as_secs_f64() >= 1.0
            {
                let effective =
                    (world - previous_world) / now.duration_since(previous_at).as_secs_f64();
                self.effective_speed = Some(effective);
                self.stalled_samples = if requested > 12.0 && effective < 0.5 {
                    self.stalled_samples.saturating_add(1)
                } else {
                    0
                };
                if self.stalled_samples >= 2 {
                    match controller.set(1) {
                        Ok(()) => self
                            .log
                            .push("World time stalled; automatically restored 1×".into()),
                        Err(error) => self.log.push(format!("Auto-restore error: {error}")),
                    }
                    self.stalled_samples = 0;
                }
                self.last_sample = Some((now, world));
            } else if self.last_sample.is_none() {
                self.last_sample = Some((now, world));
            }
        }
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("OstrivSpeed — native speed research");
            if ui.button("Refresh process and version").clicked() {
                self.refresh();
            }
            if let Some(process) = &self.process {
                ui.label(format!("PID: {}", process.pid));
                ui.label(format!("Executable: {}", process.path.display()));
                ui.label(format!("Module base: 0x{:X}", process.module_base));
            } else {
                ui.label("Ostriv is not running or cannot be inspected.");
            }
            if let Some(fp) = &self.fingerprint {
                ui.label(format!("Architecture: {} | Size: {} bytes", fp.architecture, fp.bytes));
                ui.label(format!("SHA-256: {}", fp.sha256));
                ui.label(if fp.recognized { fingerprint::KNOWN_BUILD } else { "Unknown build" });
            }
            ui.separator();
            ui.label("Experimental control: target-speed double confirmed for this exact build. 60× was disabled after the game stalled; longer-term throughput remains unverified.");
            ui.checkbox(&mut self.experimental, "Enable experimental native speed control");
            let can_attach = self.experimental && self.process.is_some() && self.fingerprint.as_ref().is_some_and(|fp| fp.recognized);
            if self.controller.is_none() {
                if ui.add_enabled(can_attach, eframe::egui::Button::new("Attach")).clicked()
                    && let (Some(game), Some(fp)) = (&self.process, &self.fingerprint)
                {
                    match SpeedController::attach(game, fp) {
                        Ok(controller) => {
                            self.controller = Some(controller);
                            self.last_sample = None;
                            self.effective_speed = None;
                            self.log.push("Attached to native target-speed variable".into());
                        }
                        Err(error) => self.log.push(error),
                    }
                }
            } else {
                if ui.button("Restore 1× and detach").clicked()
                    && let Some(mut controller) = self.controller.take()
                {
                    match controller.set(1).and_then(|()| controller.restore()) {
                        Ok(()) => self.log.push("Restored 1× and detached".into()),
                        Err(error) => self.log.push(format!("Detach restore error: {error}")),
                    }
                }
                if let Some(controller) = &self.controller {
                    match controller.read() {
                        Ok(speed) => { ui.label(format!("Native requested speed: {speed:.1}×")); }
                        Err(error) => { ui.label(error); }
                    }
                    if let Ok(current) = controller.current_speed() {
                        ui.label(format!("Native current speed: {current:.1}×"));
                    }
                    if let Some(effective) = self.effective_speed {
                        ui.label(format!("Measured world-time rate: {effective:.1}×"));
                    }
                }
            }
            ui.horizontal(|ui| {
                for speed in [1, 5, 12, 15, 20, 30, 50] {
                    let button = ui.add_enabled(self.controller.is_some(), eframe::egui::Button::new(format!("{speed}×")));
                    if button.clicked() && let Some(controller) = &mut self.controller {
                        match controller.set(speed) {
                            Ok(()) => {
                                self.last_sample = None;
                                self.effective_speed = None;
                                self.stalled_samples = 0;
                                self.log.push(format!("Requested native speed {speed}×"));
                            }
                            Err(error) => self.log.push(error),
                        }
                    }
                }
            });
            ui.label("Writes only the verified target-speed variable in the live game process. No Windows timer hook or EXE file change.");
            ui.separator();
            ui.label("Diagnostics");
            for line in self.log.iter().rev().take(12) {
                ui.label(line);
            }
        });
    }
}
