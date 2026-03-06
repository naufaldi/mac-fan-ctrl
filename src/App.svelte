<script lang="ts">
  import DesktopDashboard from './components/DesktopDashboard.svelte';
  import type { SensorData as DesignTokenSensor } from '$lib/designTokens';
  import type { FanData, SensorData } from '$lib/types';
  import { getSensors, listenToSensorUpdates } from '$lib/tauriCommands';
  import { cn } from "$lib/cn";

  let sensorData: SensorData | null = $state(null);
  let fans: DesignTokenSensor[] = $state([]);
  let unlisten: (() => void) | null = null;

  function toDesignToken(fan: FanData): DesignTokenSensor {
    return {
      id: `fan-${fan.index}`,
      fanIndex: fan.index,
      label: fan.label,
      value: Math.round(fan.actual),
      unit: 'rpm',
      status: 'normal',
      minRpm: Math.round(fan.min),
      maxRpm: Math.round(fan.max),
      targetRpm: Math.round(fan.target),
      controlMode: fan.mode === 'forced' ? 'constant' : 'auto',
    };
  }

  function applyUpdate(data: SensorData): void {
    sensorData = data;
    fans = data.fans.map(toDesignToken);
  }

  $effect(() => {
    let cancelled = false;

    const init = async () => {
      try {
        const initial = await getSensors();
        if (!cancelled) applyUpdate(initial);
      } catch (e) {
        console.error('[mac-fan-ctrl] Failed to fetch initial sensors:', e);
      }

      try {
        unlisten = await listenToSensorUpdates((data) => {
          if (!cancelled) applyUpdate(data);
        });
        if (cancelled && unlisten) { unlisten(); unlisten = null; }
      } catch (e) {
        console.error('[mac-fan-ctrl] Failed to subscribe:', e);
      }
    };

    void init();

    return () => {
      cancelled = true;
      unlisten?.();
      unlisten = null;
    };
  });
</script>

<main class={cn("flex h-full w-full")}>
  <DesktopDashboard {fans} {sensorData} />
</main>
