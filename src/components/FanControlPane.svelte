<script lang="ts">
  import { cn } from '$lib/cn';
  import { toFanRows } from '$lib/dashboardLayout';
  import type { SensorData } from '$lib/designTokens';
  import { Fan as FanIcon } from 'lucide-svelte';

  interface Props {
    fans: SensorData[];
  }

  const { fans }: Props = $props();

  const fanRows = $derived(toFanRows(fans));
  const controlBaseClass =
    'px-3 py-0.5 text-xs text-center transition-colors focus-visible:outline-none';
  const controlInactiveClass =
    'bg-(--surface-1) text-(--text-secondary) hover:bg-(--surface-hover)';
  const controlActiveClass =
    'bg-gray-400/20 dark:bg-gray-500/30 text-(--text-primary) font-medium shadow-sm';
</script>

<section class={cn("min-h-0 overflow-y-auto border-r border-(--border-subtle) bg-(--surface-1)")}>
  <!-- Header -->
  <div
    class={cn("sticky top-0 grid grid-cols-[100px_1fr_160px] items-center border-b border-(--border-subtle) bg-(--surface-2) text-[11px] font-medium text-gray-600 dark:text-gray-300")}
    role="row"
  >
    <div class="px-2 py-1 flex items-center border-r border-(--border-subtle)">Fan</div>
    <div class="px-2 py-1 flex items-center border-r border-(--border-subtle)">Min/Current/Max RPM</div>
    <div class="px-2 py-1 flex items-center">Control</div>
  </div>

  <div class={cn("flex flex-col")}>
    {#if fanRows.length === 0}
      <div class={cn("px-2 py-2 text-(--text-muted)")}>
        <span>No fan telemetry available.</span>
      </div>
    {:else}
      {#each fanRows as fan (fan.id)}
        <div
          class={cn(
            "grid grid-cols-[100px_1fr_160px] items-center odd:bg-(--surface-1) even:bg-(--surface-2) hover:bg-(--surface-hover)"
          )}
          role="row"
        >
          <div class={cn("flex min-w-0 items-center gap-2 px-2 py-1 text-[12px] text-(--text-primary)")}>
            <FanIcon size={14} class="text-(--text-secondary) shrink-0" />
            <span class="truncate">{fan.label}</span>
          </div>
          <div
            class={cn("flex items-baseline gap-1 px-2 py-1 text-[12px] font-mono [font-variant-numeric:tabular-nums]")}
          >
            <span class={cn("text-(--text-muted)")}>{fan.minRpm}</span>
            <span class="text-(--text-muted)">-</span>
            <span class={cn("font-bold text-(--text-value)")}>{fan.currentRpm ?? 'N/A'}</span>
            <span class="text-(--text-muted)">-</span>
            <span class={cn("text-(--text-muted)")}>{fan.maxRpm}</span>
          </div>
          <div class={cn("px-2 py-1 flex items-center justify-start")}>
            <div class="flex rounded-md border border-(--border-subtle) bg-(--surface-1) overflow-hidden shadow-sm">
              <button
                type="button"
                class={cn(
                  controlBaseClass,
                  "border-r border-(--border-subtle)",
                  fan.controlMode === 'auto' ? controlActiveClass : controlInactiveClass
                )}
                aria-label={`Set ${fan.label} to auto mode`}
              >
                Auto
              </button>
              <button
                type="button"
                class={cn(
                  controlBaseClass,
                  fan.controlMode === 'constant' ? controlActiveClass : controlInactiveClass
                )}
                aria-label={`Set ${fan.label} to custom mode`}
              >
                Custom...
              </button>
            </div>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</section>
