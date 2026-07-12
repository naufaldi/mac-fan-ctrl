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
import { inputClass } from "$lib/ui";

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

function formatPowerSource(source: PowerSource): string {
	switch (source) {
		case "ac":
			return "AC Power";
		case "battery":
			return "Battery";
		case "unknown":
			return "Unknown";
	}
}

const powerSourceLabel = $derived(formatPowerSource(powerSource));
</script>

<div class={cn("flex flex-col gap-3")}>
  <div class={cn("flex items-center justify-between")}>
    <span class={cn("text-[12px] font-medium text-(--text-primary)")}>
      Auto-switch presets by power source
    </span>
    <button
      type="button"
      class={cn(
        "relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring) focus-visible:ring-offset-1 focus-visible:ring-offset-(--focus-ring-offset)",
        config.enabled ? "bg-(--text-primary)" : "bg-(--border-strong)"
      )}
      role="switch"
      aria-checked={config.enabled}
      aria-label="Auto-switch presets by power source"
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
      "inline-flex items-center gap-1 rounded-full border border-(--border-subtle) bg-(--surface-0) px-2 py-0.5 text-[10px] font-medium"
    )}>
      {powerSourceLabel}
    </span>
  </div>

  {#if config.enabled}
    <div class={cn("flex flex-col gap-2")}>
      <label class={cn("flex flex-col gap-1")}>
        <span class={cn("text-[11px] text-(--text-secondary)")}>When on AC Power:</span>
        <select class={cn(inputClass, "w-full")} onchange={handleAcPreset} value={config.ac_preset ?? ""}>
          <option value="">No change</option>
          {#each presets as preset}
            <option value={preset.name}>{preset.name}</option>
          {/each}
        </select>
      </label>

      <label class={cn("flex flex-col gap-1")}>
        <span class={cn("text-[11px] text-(--text-secondary)")}>When on Battery:</span>
        <select class={cn(inputClass, "w-full")} onchange={handleBatteryPreset} value={config.battery_preset ?? ""}>
          <option value="">No change</option>
          {#each presets as preset}
            <option value={preset.name}>{preset.name}</option>
          {/each}
        </select>
      </label>
    </div>
  {/if}
</div>
