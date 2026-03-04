<script lang="ts">
  import { cn } from '$lib/cn';
  import { toFanRows } from '$lib/dashboardLayout';
  import type { SensorData } from '$lib/designTokens';

  interface Props {
    fans: SensorData[];
  }

  const { fans }: Props = $props();

  const fanRows = $derived(toFanRows(fans));
  const controlBaseClass =
    'rounded-md border px-2 py-0.5 text-left transition-colors focus-visible:outline-2 focus-visible:outline-(--focus-ring) focus-visible:outline-offset-(--focus-ring-offset)';
  const controlInactiveClass =
    'border-(--border-subtle) bg-(--surface-2) text-(--text-secondary) hover:bg-(--surface-hover)';
  const controlActiveClass =
    'border-(--control-active-border) bg-(--control-active-bg) text-(--control-active-text) font-medium';
</script>

<section class={cn("min-h-0 overflow-y-auto border-r border-(--border-subtle) p-3")}>
  <div class={cn("grid gap-1")}>
    <div
      class={cn(
        "grid grid-cols-[100px_1fr_200px] items-center gap-2 border-b border-(--border-subtle) bg-(--surface-2) px-2 py-1 font-semibold text-(--text-secondary)"
      )}
      role="row"
    >
      <span>Fan</span>
      <span>Min/Current/Max RPM</span>
      <span>Control</span>
    </div>

    {#if fanRows.length === 0}
      <div class={cn("rounded-md bg-(--surface-2) px-2 py-2 text-(--text-muted)")}>
        <span>No fan telemetry available.</span>
      </div>
    {:else}
      {#each fanRows as fan (fan.id)}
        <div
          class={cn(
            "grid grid-cols-[100px_1fr_200px] items-center gap-2 rounded-md border border-transparent px-2 py-1 odd:bg-(--surface-1) even:bg-(--surface-2) hover:border-(--border-subtle) hover:bg-(--surface-hover)"
          )}
          role="row"
        >
          <div class={cn("flex min-w-0 items-center gap-2")}>
            <span class={cn("text-(--text-muted)")} aria-hidden="true">✺</span>
            <span>{fan.label}</span>
          </div>
          <div
            class={cn("flex items-baseline gap-1 font-mono [font-variant-numeric:tabular-nums]")}
          >
            <span class={cn("text-(--text-muted)")}>{fan.minRpm}</span>
            <span>—</span>
            <span class={cn("text-base font-bold text-(--text-value)")}>{fan.currentRpm ?? 'N/A'}</span>
            <span>—</span>
            <span class={cn("text-(--text-muted)")}>{fan.maxRpm}</span>
          </div>
          <div class={cn("grid grid-cols-[auto_1fr] gap-1")}>
            <button
              type="button"
              class={cn(
                controlBaseClass,
                fan.controlMode === 'auto' ? controlActiveClass : controlInactiveClass
              )}
              aria-label={`Set ${fan.label} to auto mode (stub)`}
            >
              Auto
            </button>
            <button
              type="button"
              class={cn(
                controlBaseClass,
                fan.controlMode === 'constant' ? controlActiveClass : controlInactiveClass
              )}
              aria-label={`Set ${fan.label} to constant mode (stub)`}
            >
              Constant value of {fan.targetRpm}
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</section>
