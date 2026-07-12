<script lang="ts">
import { HardDrive, Thermometer } from "lucide-svelte";
import { cn } from "$lib/cn";
import type { Sensor } from "$lib/types";
import { telemetryGrid } from "$lib/ui";

interface Props {
	sensors: Sensor[];
	emptyLabel?: string;
}

const { sensors, emptyLabel = "No sensors available." }: Props = $props();

function statusDotClass(sensor: Sensor): string {
	if (sensor.value === null) return "bg-(--status-unknown)";
	if (sensor.sensor_type === "Storage") return "bg-(--status-normal)";
	if (sensor.value > 85) return "bg-(--status-hot)";
	if (sensor.value >= 60) return "bg-(--status-warm)";
	return "bg-(--status-normal)";
}

function statusLabel(sensor: Sensor): string {
	if (sensor.value === null) return "N/A";
	if (sensor.sensor_type === "Storage") return "OK";
	if (sensor.value > 85) return "Hot";
	if (sensor.value >= 60) return "Warm";
	return "Normal";
}

function valueTextClass(value: number | null): string {
	return value === null ? "text-(--text-muted)" : "text-(--text-value)";
}

function formatValue(sensor: Sensor): string {
	if (sensor.value === null) return "N/A";
	return sensor.unit === "C"
		? `${Math.round(sensor.value)} °C`
		: `${Math.round(sensor.value)} ${sensor.unit}`;
}

function formatRange(sensor: Sensor): string {
	if (sensor.sensor_type === "Storage") return "20 - 70";
	if (sensor.sensor_type === "Battery") return "20 - 45";
	if (sensor.sensor_type === "Gpu") return "40 - 110";
	return "40 - 100";
}

function getSensorIcon(name: string) {
	const lowerName = name.toLowerCase();
	if (
		lowerName.includes("ssd") ||
		lowerName.includes("storage") ||
		lowerName.includes("hdd") ||
		lowerName.includes("disk")
	)
		return HardDrive;
	return Thermometer;
}
</script>

{#if sensors.length === 0}
  <div class={cn("px-4 py-3 text-[12px] text-(--text-muted)")}>
    <span>{emptyLabel}</span>
  </div>
{:else}
  {#each sensors as sensor, i (sensor.key)}
    {@const Icon = getSensorIcon(sensor.name)}
    <div
      class={cn(
        "grid min-h-[30px] items-center border-b border-(--border-subtle) px-3 text-[12px] last:border-b-0",
        telemetryGrid,
        i % 2 === 1 && "bg-(--table-row-alternating)"
      )}
      role="row"
    >
      <div class={cn("flex min-w-0 items-center gap-3 pl-7 pr-3")}>
        <Icon size={18} class={cn("shrink-0 text-(--text-secondary)")} strokeWidth={1.75} />
        <span class={cn("truncate text-(--text-primary)")}>{sensor.name}</span>
      </div>
      <div class={cn("border-l border-(--table-column-separator) px-3 tabular-nums", valueTextClass(sensor.value))}>
        {formatValue(sensor)}
      </div>
      <div class={cn("border-l border-(--table-column-separator) px-3 tabular-nums text-(--text-secondary)")}>
        {formatRange(sensor)}
      </div>
      <div class={cn("border-l border-(--table-column-separator) px-3 text-(--text-secondary)")}>
        -
      </div>
      <div class={cn("flex items-center gap-1.5 border-l border-(--table-column-separator) px-3 text-(--text-secondary)")}>
        <span data-testid="status-dot" class={cn("inline-block size-1.5 shrink-0 rounded-full", statusDotClass(sensor))}></span>
        {statusLabel(sensor)}
      </div>
    </div>
  {/each}
{/if}
