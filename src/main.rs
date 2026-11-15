mod fingerprint;
mod hotkeys;
mod process;
mod simulation_speed;
mod ui;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--hotkey-check") {
        let service = hotkeys::HotkeyService::new();
        println!("{}", service.status);
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--status") {
        use process::ProcessBackend;
        match process::NativeProcessBackend.find_game() {
            Ok(Some(game)) => {
                println!("PID: {}", game.pid);
                println!("Executable: {}", game.path.display());
                match fingerprint::inspect(&game.path) {
                    Ok(fp) => {
                        println!("Architecture: {}", fp.architecture);
                        println!("Size: {} bytes", fp.bytes);
                        println!("SHA-256: {}", fp.sha256);
                        println!(
                            "Build: {}",
                            if fp.recognized {
                                fingerprint::KNOWN_BUILD
                            } else {
                                "unknown"
                            }
                        );
                        println!("Speed control: experimental, exact build only");
                    }
                    Err(error) => eprintln!("Fingerprint error: {error}"),
                }
            }
            Ok(None) => println!("Ostriv process not found"),
            Err(error) => eprintln!("Process error: {error}"),
        }
        return Ok(());
    }
    if args
        .get(1)
        .is_some_and(|arg| arg == "--experimental-speed" || arg == "--restore")
    {
        use process::ProcessBackend;
        let restoring = args[1] == "--restore";
        let speed = if restoring {
            1
        } else {
            args.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0)
        };
        let seconds = if restoring {
            0
        } else {
            args.get(3).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0)
        };
        if !restoring && !(1..=30).contains(&seconds) {
            eprintln!("Usage: --experimental-speed PRESET DURATION_SECONDS (1..30)");
            return Ok(());
        }
        let Some(game) = process::NativeProcessBackend.find_game().ok().flatten() else {
            eprintln!("Ostriv process not found");
            return Ok(());
        };
        let fp = match fingerprint::inspect(&game.path) {
            Ok(fp) => fp,
            Err(error) => {
                eprintln!("Fingerprint error: {error}");
                return Ok(());
            }
        };
        let mut controller = match simulation_speed::SpeedController::attach(&game, &fp) {
            Ok(controller) => controller,
            Err(error) => {
                eprintln!("Attach refused: {error}");
                return Ok(());
            }
        };
        match controller.set(speed) {
            Ok(()) => {
                if restoring {
                    println!("Restored: {}×", controller.read().unwrap_or(f64::NAN));
                    return Ok(());
                }
                let world_start = controller.world_time().unwrap_or(f64::NAN);
                let cpu_start = controller.cpu_seconds().unwrap_or(f64::NAN);
                let wall_start = std::time::Instant::now();
                println!(
                    "Requested {speed}×, readback {}×; holding {seconds}s",
                    controller.read().unwrap_or(f64::NAN)
                );
                let mut previous_world = world_start;
                let mut stalled_seconds = 0;
                for _ in 0..seconds {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let now = controller.world_time().unwrap_or(f64::NAN);
                    if speed > 12 && now.is_finite() && now - previous_world < 0.5 {
                        stalled_seconds += 1;
                    } else {
                        stalled_seconds = 0;
                    }
                    previous_world = now;
                    if stalled_seconds >= 2 {
                        eprintln!("World time stalled for two seconds; restoring 1× early");
                        break;
                    }
                }
                let wall_elapsed = wall_start.elapsed().as_secs_f64();
                let world_delta = controller.world_time().unwrap_or(f64::NAN) - world_start;
                let cpu_delta = controller.cpu_seconds().unwrap_or(f64::NAN) - cpu_start;
                println!(
                    "World-time delta: {world_delta:.3} units / {wall_elapsed:.3} real seconds; effective {:.2}×; process CPU {:.2} core-seconds; current speed {:.3}×",
                    world_delta / wall_elapsed,
                    cpu_delta,
                    controller.current_speed().unwrap_or(f64::NAN)
                );
                match controller.restore() {
                    Ok(()) => println!("Restored: {}×", controller.read().unwrap_or(f64::NAN)),
                    Err(error) => eprintln!("Restore error: {error}"),
                }
            }
            Err(error) => eprintln!("Speed change refused: {error}"),
        }
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1050.0, 700.0])
            .with_min_inner_size([860.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "OstrivSpeed",
        options,
        Box::new(|_cc| Ok(Box::new(ui::InspectorApp::default()))),
    )
}
