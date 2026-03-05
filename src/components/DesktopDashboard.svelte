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
    'rounded-[5px] border border-gray-300 dark:border-[#4a4a4a] bg-white dark:bg-[#3a3a3a] px-3 py-1 text-[12px] text-(--text-primary) shadow-[0_1px_2px_rgba(0,0,0,0.05)] transition-colors hover:bg-gray-50 dark:hover:bg-[#444] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500';
</script>

<section
  class={cn("flex h-full w-full flex-col overflow-hidden bg-[#ececec] dark:bg-[#1e1e1e] text-(--text-primary)")}
>
  <header
    class={cn(
      "flex shrink-0 items-center justify-center gap-2 border-b border-gray-300 dark:border-black/50 bg-[#ececec] dark:bg-[#2d2d2d] px-4 py-2"
    )}
  >
    <span class={cn("text-[12px] text-(--text-secondary)")}>Active preset:</span>
    <button class={cn(chromeButtonClass)} type="button">
      Automatic ▾
    </button>
    <button
      class={cn('w-10', chromeButtonClass)}
      type="button"
      aria-label="More options"
    >
      <span aria-hidden="true">•••</span>
    </button>
  </header>

  <div class={cn("grid min-h-0 grow grid-cols-[1fr_300px] overflow-hidden bg-white dark:bg-[#1e1e1e]")}>
    <FanControlPane fans={fanSensors} />
    <SensorListPane sensors={temperatureSensors} />
  </div>

  <footer
    class={cn(
      "flex shrink-0 items-center justify-end gap-2 border-t border-gray-300 dark:border-black/50 bg-[#ececec] dark:bg-[#2d2d2d] px-4 py-2"
    )}
  >
    <button class={cn(chromeButtonClass)} type="button">
      Hide to menu bar
    </button>
    <button class={cn(chromeButtonClass)} type="button">
      Preferences...
    </button>
    <button class={cn(chromeButtonClass, 'w-8 font-serif italic')} type="button" aria-label="Help">
      ?
    </button>
  </footer>
</section>
