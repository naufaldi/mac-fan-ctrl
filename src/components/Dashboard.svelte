<script lang="ts">
  import SensorCard from './SensorCard.svelte';
  import type { SensorData } from '$lib/designTokens';
  import { cn } from "$lib/cn";

  interface Props {
    sensors: SensorData[];
  }

  const { sensors }: Props = $props();

  const heroSensorLabels = ['CPU Package', 'GPU', 'RAM'];

  const heroSensors = $derived(
    sensors.filter((s) => heroSensorLabels.includes(s.label))
  );

  const fanSensors = $derived(
    sensors.filter((s) => s.label.includes('Fan'))
  );

  const otherSensors = $derived(
    sensors.filter(
      (s) => !heroSensorLabels.includes(s.label) && !s.label.includes('Fan')
    )
  );
</script>

<div class={cn("p-6 space-y-6")}>
  <!-- Hero Row -->
  <div class={cn("grid grid-cols-3 gap-4")}>
    {#each heroSensors as sensor (sensor.id)}
      <SensorCard {...sensor} />
    {/each}
  </div>

  <!-- Fans Section -->
  {#if fanSensors.length > 0}
    <section>
      <h2 class={cn("text-sm font-semibold text-gray-500 uppercase tracking-wide mb-3")}>
        Fans
      </h2>
      <div class={cn("grid grid-cols-2 gap-4")}>
        {#each fanSensors as sensor (sensor.id)}
          <SensorCard {...sensor} />
        {/each}
      </div>
    </section>
  {/if}

  <!-- Other Sensors -->
  {#if otherSensors.length > 0}
    <section>
      <h2 class={cn("text-sm font-semibold text-gray-500 uppercase tracking-wide mb-3")}>
        Other Sensors
      </h2>
      <div class={cn("grid grid-cols-3 gap-4")}>
        {#each otherSensors as sensor (sensor.id)}
          <SensorCard {...sensor} />
        {/each}
      </div>
    </section>
  {/if}
</div>
