mod commands;

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

fn main() {
  tauri::Builder::default()
    .setup(|app| {
      bootstrap_menu_bar(app);
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![commands::ping_backend])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
