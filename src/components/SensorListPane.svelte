<script lang="ts">
import {
	Battery,
	Cpu,
	HardDrive,
	Monitor,
	Thermometer,
	Wifi,
} from "lucide-svelte";
import { cn } from "$lib/cn";
import {
	getAllSensorsForDisplay,
	getReadMoreLabel,
	isPerCoreTemperatureUnavailable,
	SUMMARY_SENSOR_LIMIT,
	shouldShowReadMore,
} from "$lib/sensorListPaneState";
import type { Sensor, SensorData } from "$lib/types";

interface Props {
	sensorData: SensorData | null;
}

let { sensorData }: Props = $props();

const displaySensors = $derived(
	sensorData ? getAllSensorsForDisplay(sensorData) : [],
);
const perCoreTemperatureUnavailable = $derived(
	isPerCoreTemperatureUnavailable(sensorData),
);
const loading = $derived(!sensorData);
let expanded: boolean = $state(false);
const showReadMore = $derived(shouldShowReadMore(displaySensors.length));
const visibleSensors = $derived(
	!expanded && showReadMore
		? displaySensors.slice(0, SUMMARY_SENSOR_LIMIT)
		: displaySensors,
);
const cnClasses = (
	...inputs: Array<string | undefined | null | false>
): string => cn(...inputs);

function getStatusColor(value: number | null): string {
	if (value === null) return "text-(--text-muted)";
	return "text-gray-900 dark:text-gray-100";
}

function getStatusDotColor(value: number | null): string {
	if (value === null) return "bg-gray-400";
	if (value > 85) return "bg-red-500";
	if (value >= 70) return "bg-yellow-500";
	return "bg-green-500";
}

function formatValue(sensor: Sensor): string {
	if (sensor.name === "Disk Drives:") {
		return "";
	}

	if (sensor.value === null) {
		return "N/A";
	}

	return `${Math.round(sensor.value)}`;
}

function getSensorIcon(name: string) {
	const lowerName = name.toLowerCase();
	if (name === "Disk Drives:") return null;
	if (lowerName.includes("cpu")) return Cpu;
	if (lowerName.includes("gpu")) return Monitor;
	if (lowerName.includes("ram") || lowerName.includes("memory")) return Monitor; // Fallback since MemoryStick isn't always available, let's use Thermometer or something else
	if (
		lowerName.includes("ssd") ||
		lowerName.includes("storage") ||
		lowerName.includes("hdd")
	)
		return HardDrive;
	if (lowerName.includes("battery")) return Battery;
	if (
		lowerName.includes("airport") ||
		lowerName.includes("wi-fi") ||
		lowerName.includes("wifi")
	)
		return Wifi;
	return Thermometer;
}

function getSensorIconColor(name: string) {
	const lowerName = name.toLowerCase();
	if (name === "Disk Drives:") return "text-(--text-muted)";
	if (lowerName.includes("cpu")) return "text-green-600 dark:text-green-500";
	if (lowerName.includes("gpu")) return "text-green-600 dark:text-green-500";
	if (lowerName.includes("ram") || lowerName.includes("memory"))
		return "text-green-600 dark:text-green-500";
	if (
		lowerName.includes("ssd") ||
		lowerName.includes("storage") ||
		lowerName.includes("hdd")
	)
		return "text-gray-600 dark:text-gray-400";
	if (lowerName.includes("battery")) return "text-gray-600 dark:text-gray-400";
	if (
		lowerName.includes("airport") ||
		lowerName.includes("wi-fi") ||
		lowerName.includes("wifi")
	)
		return "text-blue-500";
	return "text-gray-500";
}
</script>

<aside class={cnClasses("min-h-0 overflow-y-auto bg-(--surface-1)")} aria-label="Temperature sensors panel">
  <!-- Header -->
  <div
    class={cnClasses("sticky top-0 grid grid-cols-[1fr_auto] items-center border-b border-(--border-subtle) bg-(--surface-2) text-[11px] font-medium text-gray-600 dark:text-gray-300")}
    role="row"
  >
    <div class="px-2 py-1 flex items-center border-r border-(--border-subtle)">Sensor</div>
    <div class="px-2 py-1 min-w-[70px] text-right">Value °C</div>
  </div>

  {#if loading}
    <div class={cnClasses("px-2 py-2 text-(--text-muted)")}>
      <span>Loading sensors...</span>
    </div>
  {:else if displaySensors.length > 0}
    <div class={cnClasses("flex flex-col")}>
      {#if perCoreTemperatureUnavailable}
        <div class={cnClasses("border-b border-(--border-subtle) bg-(--surface-2) px-2 py-1 text-[11px] text-(--text-muted)")}>
          <span>Per-core temperature not exposed on this Mac</span>
        </div>
      {/if}
      {#each visibleSensors as sensor (sensor.key)}
        {@const Icon = getSensorIcon(sensor.name)}
        <div
          class={cnClasses(
            "grid grid-cols-[1fr_auto] items-center odd:bg-(--surface-1) even:bg-(--surface-2) hover:bg-(--surface-hover)"
          )}
          role="row"
        >
          <div class={cnClasses("flex items-center gap-2 truncate px-2 py-1 text-[12px]", sensor.name === "Disk Drives:" ? "text-(--text-muted) font-medium" : "text-(--text-primary)")}>
            {#if Icon}
              <Icon size={14} class={cnClasses("shrink-0", getSensorIconColor(sensor.name))} />
            {/if}
            <span class="truncate">{sensor.name}</span>
          </div>
          <div class={cnClasses("flex items-center justify-end gap-1.5 px-2 py-1 min-w-[70px] text-right text-[12px] font-mono [font-variant-numeric:tabular-nums]", getStatusColor(sensor.value))}>
            {#if sensor.value !== null && sensor.name !== "Disk Drives:"}
              <span class={cnClasses("inline-block size-1.5 shrink-0 rounded-full", getStatusDotColor(sensor.value))}></span>
            {/if}
            {formatValue(sensor)}
          </div>
        </div>
      {/each}
      {#if showReadMore}
        <button
          type="button"
          class={cnClasses(
            "w-full border-t border-(--border-subtle) bg-(--surface-2) px-2 py-1.5 text-[11px] text-(--text-muted) hover:text-(--text-secondary) hover:bg-(--surface-hover) cursor-pointer transition-colors"
          )}
          onclick={() => { expanded = !expanded; }}
        >
          {getReadMoreLabel(expanded, displaySensors.length)}
        </button>
      {/if}
    </div>
  {:else}
    <div class={cnClasses("px-2 py-2 text-(--text-muted)")}>
      <span>No sensors available.</span>
    </div>
  {/if}
</aside>
