use crate::smc::SensorService;

pub const SENSOR_UPDATE_EVENT: &str = "sensor_update";

#[tauri::command]
pub fn ping_backend(message: String) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("message must not be empty".to_string());
    }

    Ok(format!("Hello from Rust: {message}"))
}

#[tauri::command]
pub fn get_sensors() -> Result<crate::smc::SensorData, String> {
    let mut service = SensorService::new();
    service.read_all_sensors()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::ping_backend;

    #[test]
    fn ping_backend_returns_expected_payload() {
        let result = ping_backend("Hello world".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or_default(), "Hello from Rust: Hello world");
    }

    #[test]
    fn ping_backend_rejects_empty_message() {
        let result = ping_backend(String::new());
        assert!(result.is_err());
    }
}
