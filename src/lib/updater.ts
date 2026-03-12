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
		const raw = error instanceof Error ? error.message : String(error);
		const isNoRelease =
			raw.includes("Could not fetch") ||
			raw.includes("404") ||
			raw.includes("PLACEHOLDER");
		const message = isNoRelease
			? "No updates available yet. This is a development build."
			: raw;
		return { status: isNoRelease ? "up-to-date" : "error", message };
	}
}
