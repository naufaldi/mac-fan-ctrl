<script lang="ts">
import { Fan as FanIcon } from "lucide-svelte";
import { cn } from "$lib/cn";
import { toFanRows } from "$lib/dashboardLayout";
import type { SensorData } from "$lib/designTokens";
import { telemetryGrid } from "$lib/ui";

interface Props {
	fans: SensorData[];
	emptyLabel?: string;
}

const { fans, emptyLabel = "No fan telemetry available." }: Props = $props();

const fanRows = $derived(toFanRows(fans));

const formatRpm = (value: number | null): string =>
	value === null
		? "N/A"
		: `${new Intl.NumberFormat("en-US").format(Math.round(value))} RPM`;

const modeLabel = (mode: string): string =>
	mode === "constant" ? "Synced" : "Auto";
</script>

{#if fanRows.length === 0}
  <div class={cn("px-4 py-3 text-[12px] text-(--text-muted)")}>
    <span>{emptyLabel}</span>
  </div>
{:else}
  {#each fanRows as fan, i (fan.id)}
    <div
      class={cn(
        "grid min-h-[30px] items-center border-b border-(--border-subtle) px-3 text-[12px] last:border-b-0",
        telemetryGrid,
        i % 2 === 1 && "bg-(--table-row-alternating)"
      )}
      role="row"
    >
      <div class={cn("flex min-w-0 items-center gap-3 pl-7 pr-3")}>
        <FanIcon size={18} class={cn("shrink-0 text-(--text-secondary)")} strokeWidth={1.75} />
        <span class={cn("truncate text-(--text-primary)")}>{fan.label}</span>
      </div>
      <div class={cn("border-l border-(--table-column-separator) px-3 tabular-nums text-(--text-value)")}>
        {formatRpm(fan.currentRpm)}
      </div>
      <div class={cn("border-l border-(--table-column-separator) px-3 tabular-nums text-(--text-secondary)")}>
        {fan.minRpm} - {fan.maxRpm}
      </div>
      <div class={cn("border-l border-(--table-column-separator) px-3 text-(--text-secondary)")}>
        {modeLabel(fan.controlMode)}
      </div>
      <div class={cn("flex items-center gap-1.5 border-l border-(--table-column-separator) px-3 text-(--text-secondary)")}>
        <span class={cn("inline-block size-1.5 shrink-0 rounded-full bg-(--status-normal)")}></span>
        OK
      </div>
    </div>
  {/each}
{/if}
