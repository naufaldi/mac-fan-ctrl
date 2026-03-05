mod commands;
mod smc;

use std::thread;
use std::time::Duration;
use tauri::Emitter;

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

fn start_sensor_stream(app_handle: tauri::AppHandle) {
	thread::spawn(move || {
		let mut client: Option<smc::SmcClient> = smc::SmcClient::new().ok();

		loop {
			if client.is_none() {
				client = smc::SmcClient::new().ok();
			}

			if let Some(active_client) = client.as_mut() {
				match active_client.read_all_sensors() {
					Ok(sensor_data) => {
						if let Err(error) =
							app_handle.emit(commands::SENSOR_UPDATE_EVENT, sensor_data)
						{
							eprintln!("[mac-fan-ctrl] Failed to emit sensor_update event: {error}");
						}
					}
					Err(error) => {
						eprintln!("[mac-fan-ctrl] Sensor stream read failed: {error}");
						client = None;
					}
				}
			}

			thread::sleep(Duration::from_millis(1500));
		}
	});
}

fn main() {
	tauri::Builder::default()
		.setup(|app| {
			bootstrap_menu_bar(app);
			start_sensor_stream(app.handle().clone());
			Ok(())
		})
		.invoke_handler(tauri::generate_handler![
			commands::ping_backend,
			commands::get_sensors,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
