import { invoke } from "@tauri-apps/api/core";

export async function pingBackend(message: string): Promise<string> {
	return invoke<string>("ping_backend", { message });
}
