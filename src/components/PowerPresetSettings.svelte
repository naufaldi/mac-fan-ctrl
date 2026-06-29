<script lang="ts">
import { cn } from "$lib/cn";
import {
	getCurrentPowerSource,
	getPowerPresetConfig,
	getPresets,
	listenToPowerSourceChanges,
	setPowerPresetConfig,
} from "$lib/tauriCommands";
import type { PowerPresetConfig, PowerSource, Preset } from "$lib/types";

let config: PowerPresetConfig = $state({
	enabled: false,
	ac_preset: null,
	battery_preset: null,
});
let powerSource: PowerSource = $state("unknown");
let presets: Preset[] = $state([]);

$effect(() => {
	getPowerPresetConfig()
		.then((c) => {
			config = c;
		})
		.catch(() => {});
	getCurrentPowerSource()
		.then((s) => {
			powerSource = s;
		})
		.catch(() => {});
	getPresets()
		.then((p) => {
			presets = p;
		})
		.catch(() => {});
});

$effect(() => {
	const unlistenPromise = listenToPowerSourceChanges((s) => {
		powerSource = s;
	});
	return () => {
		unlistenPromise.then((unlisten) => unlisten());
	};
});

async function handleToggle(): Promise<void> {
	try {
		config = await setPowerPresetConfig({ enabled: !config.enabled });
	} catch (error) {
		console.error("[PowerPresetSettings] toggle failed:", error);
	}
}

async function handleAcPreset(event: Event): Promise<void> {
	const target = event.target as HTMLSelectElement;
	const value = target.value === "" ? null : target.value;
	try {
		config = await setPowerPresetConfig({ ac_preset: value });
	} catch (error) {
		console.error("[PowerPresetSettings] set AC preset failed:", error);
	}
}

async function handleBatteryPreset(event: Event): Promise<void> {
	const target = event.target as HTMLSelectElement;
	const value = target.value === "" ? null : target.value;
	try {
		config = await setPowerPresetConfig({ battery_preset: value });
	} catch (error) {
		console.error("[PowerPresetSettings] set Battery preset failed:", error);
	}
}

const powerSourceLabel = $derived(
	powerSource === "ac"
		? "AC Power"
		: powerSource === "battery"
			? "Battery"
			: "Unknown",
);

const selectClass =
	"w-full rounded-(--radius-input) border border-(--border-subtle) bg-(--surface-elevated) px-2 py-1 text-[12px] text-(--text-primary) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring)";
</script>

<div class={cn("flex flex-col gap-3")}>
  <div class={cn("flex items-center justify-between")}>
    <span class={cn("text-[12px] font-medium text-(--text-primary)")}>
      Auto-switch presets by power source
    </span>
    <button
      type="button"
      class={cn(
        "relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
        config.enabled ? "bg-(--control-active-bg)" : "bg-(--color-silver-mist)"
      )}
      role="switch"
      aria-checked={config.enabled}
      aria-label="Toggle auto-switch presets by power source"
      onclick={handleToggle}
    >
      <span
        class={cn(
          "pointer-events-none inline-block h-4 w-4 rounded-full bg-(--surface-elevated) shadow transition-transform",
          config.enabled ? "translate-x-4" : "translate-x-0"
        )}
      ></span>
    </button>
  </div>

  <div class={cn("flex items-center gap-2 text-[11px] text-(--text-secondary)")}>
    <span class={cn(
      "inline-flex items-center gap-1 rounded-(--radius-capsule) border border-(--border-subtle) bg-(--surface-2) px-2 py-0.5 text-[10px] font-medium text-(--text-secondary)"
    )}>
      {powerSourceLabel}
    </span>
  </div>

  {#if config.enabled}
    <div class={cn("flex flex-col gap-2")}>
      <label class={cn("flex flex-col gap-1")}>
        <span class={cn("text-[11px] text-(--text-secondary)")}>When on AC Power:</span>
        <select class={cn(selectClass)} onchange={handleAcPreset} value={config.ac_preset ?? ""}>
          <option value="">No change</option>
          {#each presets as preset}
            <option value={preset.name}>{preset.name}</option>
          {/each}
        </select>
      </label>

      <label class={cn("flex flex-col gap-1")}>
        <span class={cn("text-[11px] text-(--text-secondary)")}>When on Battery:</span>
        <select class={cn(selectClass)} onchange={handleBatteryPreset} value={config.battery_preset ?? ""}>
          <option value="">No change</option>
          {#each presets as preset}
            <option value={preset.name}>{preset.name}</option>
          {/each}
        </select>
      </label>
    </div>
  {/if}
</div>
