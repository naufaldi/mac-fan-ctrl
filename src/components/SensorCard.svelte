<script lang="ts">
  import StatusDot from './StatusDot.svelte';
  import Sparkline from './Sparkline.svelte';
  import type { TemperatureStatus } from '$lib/designTokens';
  import { formatTemperature, formatRpm } from '$lib/format';
  import { cn } from "$lib/cn";

  interface Props {
    label: string;
    value: number | null;
    unit: 'celsius' | 'rpm';
    status: TemperatureStatus;
    sparklineData?: number[];
  }

  const { label, value, unit, status, sparklineData }: Props = $props();

  const formattedValue = $derived(
    value === null
      ? 'N/A'
      : unit === 'celsius'
        ? formatTemperature(value)
        : formatRpm(value)
  );

  const statusColor = $derived(`var(--color-status-${status})`);
</script>

<div class={cn("bg-(--color-surface-card) hover:bg-(--color-surface-hover) rounded-(--radius-card) p-4 transition-colors")}>
  <div class={cn("flex items-center justify-between mb-2")}>
    <span class={cn("text-sm font-medium text-gray-700 dark:text-gray-200")}>{label}</span>
    <StatusDot {status} />
  </div>

  <div class={cn("font-mono-numeric text-2xl font-semibold")} style="color: {statusColor}">
    {formattedValue}
  </div>

  {#if sparklineData && sparklineData.length > 0}
    <div class={cn("mt-2")}>
      <Sparkline data={sparklineData} {statusColor} />
    </div>
  {/if}
</div>
