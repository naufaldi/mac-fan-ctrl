<script lang="ts">
  import { cn } from '$lib/cn';
  import type { SensorData } from '$lib/designTokens';
  import { partitionSensors } from '$lib/dashboardLayout';
  import FanControlPane from './FanControlPane.svelte';
  import SensorListPane from './SensorListPane.svelte';

  interface Props {
    sensors: SensorData[];
  }

  const { sensors }: Props = $props();

  const sensorPartitions = $derived(partitionSensors(sensors));
  const fanSensors = $derived(sensorPartitions.fanSensors);
  const temperatureSensors = $derived(sensorPartitions.temperatureSensors);
  const chromeButtonClass =
    'rounded-md border border-(--border-strong) bg-(--surface-muted) px-3 py-1 text-(--text-primary) transition-colors hover:bg-(--surface-hover) focus-visible:outline-2 focus-visible:outline-(--focus-ring) focus-visible:outline-offset-(--focus-ring-offset)';
</script>

<section
  class={cn("flex h-full w-full flex-col overflow-hidden bg-(--surface-1) text-(--text-primary)")}
>
  <header
    class={cn(
      "flex shrink-0 items-center justify-center gap-2 border-b border-(--border-subtle) bg-(--surface-1) px-4 py-2"
    )}
  >
    <span class={cn("text-(--text-secondary)")}>Active preset:</span>
    <button class={cn(chromeButtonClass)} type="button">
      Custom* ▾
    </button>
    <button
      class={cn('w-10', chromeButtonClass)}
      type="button"
      aria-label="More options"
    >
      <span aria-hidden="true">•••</span>
    </button>
  </header>

  <div class={cn("grid min-h-0 grow grid-cols-[1fr_300px] overflow-hidden")}>
    <FanControlPane fans={fanSensors} />
    <SensorListPane sensors={temperatureSensors} />
  </div>

  <footer
    class={cn(
      "flex shrink-0 items-center justify-end gap-2 border-t border-(--border-subtle) bg-(--surface-1) px-4 py-2"
    )}
  >
    <button class={cn(chromeButtonClass)} type="button">
      Hide to menu bar
    </button>
    <button class={cn(chromeButtonClass)} type="button">
      Preferences...
    </button>
    <button class={cn(chromeButtonClass)} type="button" aria-label="Help">
      ?
    </button>
  </footer>
</section>
