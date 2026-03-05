<script lang="ts">
  import DesktopDashboard from './components/DesktopDashboard.svelte';
  import type { TemperatureStatus, SensorData } from '$lib/designTokens';
  import { cn } from "$lib/cn";

  const mockFans: SensorData[] = [
    {
      id: 'fan-left',
      label: 'Left Fan',
      value: 2450,
      unit: 'rpm',
      status: 'normal' as TemperatureStatus,
      minRpm: 1200,
      maxRpm: 5779,
      controlMode: 'constant',
      targetRpm: 3000,
      sparklineData: [2000, 2100, 2200, 2350, 2450],
    },
    {
      id: 'fan-right',
      label: 'Right Fan',
      value: 2300,
      unit: 'rpm',
      status: 'normal' as TemperatureStatus,
      minRpm: 1200,
      maxRpm: 6241,
      controlMode: 'constant',
      targetRpm: 3000,
    },
  ];
</script>

<main class={cn("flex h-full w-full")}>
  <DesktopDashboard sensors={mockFans} />
</main>
