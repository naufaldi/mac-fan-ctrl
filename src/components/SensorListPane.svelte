<script lang="ts">
  import { formatSensorValue } from '$lib/dashboardLayout';
  import StatusDot from './StatusDot.svelte';
  import type { SensorData } from '$lib/designTokens';
  import { cn } from "$lib/cn";

  interface Props {
    sensors: SensorData[];
  }

  const { sensors }: Props = $props();
</script>

<aside class={cn("min-h-0 overflow-y-auto p-3")} aria-label="Temperature sensors panel">
  <div class={cn("grid gap-1")}>
    <div
      class={cn("grid grid-cols-[1fr_auto] items-center gap-2 border-b border-(--border-subtle) bg-(--surface-2) px-2 py-1 font-semibold text-(--text-secondary)")}
      role="row"
    >
      <span>Sensor</span>
      <span class={cn("text-right")}>Value °C</span>
    </div>

    {#if sensors.length === 0}
      <div class={cn("rounded-md bg-(--surface-2) px-2 py-2 text-(--text-muted)")}>
        <span>No sensors available.</span>
      </div>
    {:else}
      {#each sensors as sensor (sensor.id)}
        <div
          class={cn("grid grid-cols-[1fr_auto] items-center gap-2 rounded-md border border-transparent px-2 py-1 odd:bg-(--surface-1) even:bg-(--surface-2) hover:border-(--border-subtle) hover:bg-(--surface-hover)")}
          role="row"
        >
          <div class={cn("flex min-w-0 items-center gap-2")}>
            <StatusDot status={sensor.status} />
            <span class={cn("truncate")}>{sensor.label}</span>
          </div>
          <span class={cn("text-right font-mono text-(--text-value) [font-variant-numeric:tabular-nums]")}>
            {formatSensorValue(sensor)}
          </span>
        </div>
      {/each}
    {/if}
  </div>
</aside>
