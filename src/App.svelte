<script lang="ts">
  import DesktopDashboard from './components/DesktopDashboard.svelte';
  import type { TemperatureStatus, SensorData } from '$lib/designTokens';
  import { cn } from "$lib/cn";

  // Mock data for initial implementation
  const mockSensors: SensorData[] = [
    {
      id: 'cpu',
      label: 'CPU Package',
      value: 72,
      unit: 'celsius',
      status: 'warm' as TemperatureStatus,
      sparklineData: [68, 69, 70, 71, 72, 71, 72, 73, 72],
    },
    {
      id: 'gpu',
      label: 'GPU',
      value: 68,
      unit: 'celsius',
      status: 'normal' as TemperatureStatus,
    },
    {
      id: 'ram',
      label: 'RAM',
      value: 45,
      unit: 'celsius',
      status: 'normal' as TemperatureStatus,
    },
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
    {
      id: 'battery',
      label: 'Battery',
      value: 35,
      unit: 'celsius',
      status: 'normal' as TemperatureStatus,
    },
    {
      id: 'ssd',
      label: 'SSD',
      value: 42,
      unit: 'celsius',
      status: 'normal' as TemperatureStatus,
    },
  ];
</script>

<main class={cn("flex h-full w-full")}>
  <DesktopDashboard sensors={mockSensors} />
</main>
