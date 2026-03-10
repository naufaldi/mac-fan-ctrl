import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";

// ── Types ────────────────────────────────────────────────────────────────────

interface UpdateAvailable {
	readonly status: "available";
	readonly version: string;
	readonly body: string | null;
	readonly download: () => Promise<void>;
}

interface UpToDate {
	readonly status: "up-to-date";
}

interface UpdateError {
	readonly status: "error";
	readonly message: string;
}

export type UpdateCheckResult = UpdateAvailable | UpToDate | UpdateError;

// ── Public API ───────────────────────────────────────────────────────────────

export async function checkForUpdate(): Promise<UpdateCheckResult> {
	try {
		const update = await check();

		if (!update) {
			return { status: "up-to-date" };
		}

		return {
			status: "available",
			version: update.version,
			body: update.body ?? null,
			download: async () => {
				await update.downloadAndInstall();
				await relaunch();
			},
		};
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		return { status: "error", message };
	}
}
