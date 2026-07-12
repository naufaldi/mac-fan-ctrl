import type { FanControlConfig, SensorData } from "$lib/types";

type SensorListener = (payload: { payload: SensorData }) => void;

type E2eMockState = {
	fanState: {
		mode: "auto" | "forced";
		targetRpm: number;
		min: number;
		max: number;
	};
	listeners: Map<string, Set<SensorListener>>;
	lastCommand?: string;
};

const getMockState = (): E2eMockState => {
	const globalState = globalThis as { __fanguardE2eMock?: E2eMockState };
	if (!globalState.__fanguardE2eMock) {
		globalState.__fanguardE2eMock = {
			fanState: {
				mode: "auto",
				targetRpm: 3275,
				min: 1200,
				max: 6550,
			},
			listeners: new Map<string, Set<SensorListener>>(),
		};
	}
	return globalState.__fanguardE2eMock;
};

const isAppStoreDistribution = (): boolean =>
	import.meta.env.VITE_FANGUARD_DISTRIBUTION === "app-store";

const baseSensorData = (state: E2eMockState): SensorData => ({
	summary: {
		cpu_package: {
			key: "TC0D",
			name: "CPU Die",
			value: 55,
			unit: "C",
			sensor_type: "Cpu",
		},
		gpu: null,
		ram: null,
		ssd: null,
	},
	details: [
		{
			key: "TC0D",
			name: "CPU Die",
			value: 55,
			unit: "C",
			sensor_type: "Cpu",
		},
		{
			key: "TB0T",
			name: "Battery",
			value: 37,
			unit: "C",
			sensor_type: "Battery",
			source: "smc",
		},
		{
			key: "TG0D",
			name: "GPU Cluster (AGX)",
			value: 68,
			unit: "C",
			sensor_type: "Gpu",
			source: "iohid_iokit",
		},
	],
	diagnostics: {
		model_id: "MacBookPro18,3",
		diagnostics_enabled: false,
		active_providers: ["smc", "iohid_iokit"],
		unresolved: [],
		per_core_cpu_temp_available: true,
	},
	fans: [
		{
			index: 0,
			label: "System Fan 1",
			actual: 2000,
			min: state.fanState.min,
			max: state.fanState.max,
			target:
				state.fanState.mode === "forced"
					? state.fanState.targetRpm
					: state.fanState.min,
			mode: state.fanState.mode,
		},
		{
			index: 1,
			label: "System Fan 2",
			actual: 2200,
			min: state.fanState.min,
			max: state.fanState.max,
			target: state.fanState.targetRpm,
			mode: state.fanState.mode,
		},
		{
			index: 2,
			label: "System Fan 3",
			actual: 2500,
			min: state.fanState.min,
			max: state.fanState.max,
			target: state.fanState.targetRpm,
			mode: state.fanState.mode,
		},
	],
});

const emitSensorUpdate = (state: E2eMockState): void => {
	const payload = baseSensorData(state);
	state.listeners.get("sensor_update")?.forEach((handler) => {
		try {
			handler({ payload });
		} catch (error) {
			console.error("[e2e-mock] sensor_update handler failed:", error);
		}
	});
};

export async function invoke<T>(
	cmd: string,
	args?: Record<string, unknown>,
): Promise<T> {
	const state = getMockState();

	switch (cmd) {
		case "get_sensors":
			return baseSensorData(state) as T;
		case "get_fan_control_configs":
			return (state.fanState.mode === "forced"
				? {
						"0": {
							mode: "constant_rpm",
							target_rpm: state.fanState.targetRpm,
						},
					}
				: {}) as T;
		case "set_fan_constant_rpm": {
			const rpm = Number(args?.rpm ?? state.fanState.targetRpm);
			state.fanState.mode = "forced";
			state.fanState.targetRpm = Math.max(
				state.fanState.min,
				Math.min(state.fanState.max, Math.round(rpm)),
			);
			state.lastCommand = "set_fan_constant_rpm";
			emitSensorUpdate(state);
			return undefined as T;
		}
		case "set_fan_auto": {
			state.fanState.mode = "auto";
			state.lastCommand = "set_fan_auto";
			emitSensorUpdate(state);
			return undefined as T;
		}
		case "set_all_fans_constant_rpm": {
			const rpm = Number(args?.rpm ?? state.fanState.targetRpm);
			state.fanState.mode = "forced";
			state.fanState.targetRpm = Math.max(
				state.fanState.min,
				Math.min(state.fanState.max, Math.round(rpm)),
			);
			state.lastCommand = "set_all_fans_constant_rpm";
			emitSensorUpdate(state);
			return undefined as T;
		}
		case "restore_all_fans_auto":
			state.fanState.mode = "auto";
			state.lastCommand = "restore_all_fans_auto";
			emitSensorUpdate(state);
			return undefined as T;
		case "get_alert_config":
			return {
				enabled: false,
				cpu_threshold: 90,
				cooldown_secs: 60,
			} as T;
		case "get_presets":
			return [] as T;
		case "get_active_preset":
			return (state.fanState.mode === "auto" ? "Automatic" : null) as T;
		case "get_power_preset_config":
			return {
				enabled: false,
				ac_preset: null,
				battery_preset: null,
			} as T;
		case "get_current_power_source":
			return "unknown" as T;
		case "get_privilege_status":
			return {
				has_write_access: !isAppStoreDistribution(),
				fan_control_available: !isAppStoreDistribution(),
				reason: isAppStoreDistribution()
					? "Fan control is disabled in TestFlight builds."
					: null,
			} as T;
		case "get_tray_display_mode":
			return 0 as T;
		default:
			throw new Error(`Unhandled e2e mock command: ${cmd}`);
	}
}

export async function listen<T>(
	event: string,
	handler: (event: { payload: T }) => void,
): Promise<() => void> {
	const state = getMockState();
	const wrapped = handler as SensorListener;
	const bucket = state.listeners.get(event) ?? new Set<SensorListener>();
	bucket.add(wrapped);
	state.listeners.set(event, bucket);

	return () => {
		bucket.delete(wrapped);
	};
}

export async function emit<T>(event: string, payload?: T): Promise<void> {
	getMockState().listeners.get(event)?.forEach((handler) => {
		handler({ payload } as { payload: SensorData });
	});
}

export function isTauri(): boolean {
	return true;
}

export type { FanControlConfig };

export class Resource {
	close = async (): Promise<void> => {};
}

export class Channel<T = unknown> {
	onmessage: ((message: T) => void) | undefined;
}

export const transformCallback = (): number => 0;
