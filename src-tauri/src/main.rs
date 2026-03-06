mod apple_silicon_sensors;
mod commands;
mod fan_control;
mod presets;
mod smc;
mod smc_writer;

use std::thread;
use std::time::{Duration, Instant};

use tauri::{Emitter, Manager};

use commands::AppState;

fn bootstrap_menu_bar(app: &mut tauri::App) {
	println!("[mac-fan-ctrl] Starting shell bootstrap");

	match tauri::menu::Menu::default(app.handle()) {
		Ok(menu) => match app.set_menu(menu) {
			Ok(_) => println!("[mac-fan-ctrl] Menu bar bootstrapped successfully"),
			Err(error) => {
				eprintln!(
					"[mac-fan-ctrl] Menu bar setup failed, continuing without custom menu: {error}"
				);
			}
		},
		Err(error) => {
			eprintln!(
				"[mac-fan-ctrl] Menu baseline not available, continuing with defaults: {error}"
			);
		}
	}

	println!("[mac-fan-ctrl] Shell bootstrap complete");
}

fn run_fan_control_tick(app_handle: &tauri::AppHandle, sensor_data: &smc::SensorData) {
	let state = app_handle.state::<AppState>();
	if let Ok(writer_guard) = state.smc_writer.lock() {
		if let Some(writer) = writer_guard.as_ref() {
			if let Ok(mut control) = state.fan_control.lock() {
				if let Err(error) = control.tick(
					&sensor_data.details,
					&sensor_data.fans,
					writer,
				) {
					eprintln!("[mac-fan-ctrl] Fan control tick failed: {error}");
				}
			}
		}
	};
}

fn start_sensor_stream(app_handle: tauri::AppHandle) {
	thread::spawn(move || {
		let mut service = smc::SensorService::new();
		let fast_interval = Duration::from_millis(1000);
		let full_read_every = 3; // Do a full sensor read every Nth cycle
		let mut cycle_count: u32 = 0;
		let mut last_full_data: Option<smc::SensorData> = None;

		loop {
			let cycle_start = Instant::now();
			let is_full_cycle = cycle_count % full_read_every == 0;

			if is_full_cycle {
				// Full read: temperatures + fans (slow due to all_data() scan)
				match service.read_all_sensors() {
					Ok(sensor_data) => {
						run_fan_control_tick(&app_handle, &sensor_data);

						if let Err(error) =
							app_handle.emit(commands::SENSOR_UPDATE_EVENT, &sensor_data)
						{
							eprintln!("[mac-fan-ctrl] Failed to emit sensor_update event: {error}");
						}
						last_full_data = Some(sensor_data);
					}
					Err(error) => {
						eprintln!("[mac-fan-ctrl] Sensor stream read failed: {error}");
						service = smc::SensorService::new();
					}
				}
			} else if let Some(ref mut cached) = last_full_data {
				// Fast path: only re-read fan data (~10 key reads, <50ms)
				let fresh_fans = service.read_fans_only();
				if !fresh_fans.is_empty() {
					cached.fans = fresh_fans;
				}

				run_fan_control_tick(&app_handle, cached);

				if let Err(error) =
					app_handle.emit(commands::SENSOR_UPDATE_EVENT, &*cached)
				{
					eprintln!("[mac-fan-ctrl] Failed to emit sensor_update event: {error}");
				}
			}

			cycle_count = cycle_count.wrapping_add(1);

			let elapsed = cycle_start.elapsed();
			if let Some(remaining) = fast_interval.checked_sub(elapsed) {
				thread::sleep(remaining);
			}
		}
	});
}

fn restore_fans_on_close(window: &tauri::Window) {
	let state = window.state::<AppState>();
	let Ok(writer_guard) = state.smc_writer.lock() else { return };
	let Some(writer) = writer_guard.as_ref() else { return };
	let Ok(mut control) = state.fan_control.lock() else { return };
	eprintln!("[mac-fan-ctrl] Window closing — restoring all fans to Auto");
	control.restore_all_auto(writer);
}

fn main() {
	tauri::Builder::default()
		.manage(AppState::new())
		.setup(|app| {
			bootstrap_menu_bar(app);
			start_sensor_stream(app.handle().clone());
			Ok(())
		})
		.invoke_handler(tauri::generate_handler![
			commands::ping_backend,
			commands::get_sensors,
			commands::set_fan_constant_rpm,
			commands::set_fan_sensor_control,
			commands::set_fan_auto,
			commands::get_fan_control_configs,
			commands::get_presets,
			commands::get_active_preset,
			commands::apply_preset,
			commands::save_preset,
			commands::delete_preset,
			commands::get_privilege_status,
			commands::request_privilege_restart,
		])
		.on_window_event(|window, event| {
			if let tauri::WindowEvent::Destroyed = event {
				restore_fans_on_close(window);
			}
		})
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
