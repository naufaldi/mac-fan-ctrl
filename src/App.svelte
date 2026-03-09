<script lang="ts">
  import DesktopDashboard from './components/DesktopDashboard.svelte';
  import type { SensorData as DesignTokenSensor } from '$lib/designTokens';
  import type { FanData, SensorData } from '$lib/types';
  import { getSensors, listenToSensorUpdates } from '$lib/tauriCommands';
  import { cn } from "$lib/cn";

  type AppStatus =
    | { readonly kind: 'loading' }
    | { readonly kind: 'error'; readonly message: string }
    | { readonly kind: 'ready' };

  let appStatus: AppStatus = $state({ kind: 'loading' });
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
    appStatus = { kind: 'ready' };
  }

  $effect(() => {
    let cancelled = false;

    const init = async () => {
      try {
        const initial = await getSensors();
        if (!cancelled) applyUpdate(initial);
      } catch (e) {
        const message = e instanceof Error ? e.message : String(e);
        console.error('[mac-fan-ctrl] Failed to fetch initial sensors:', e);
        if (!cancelled) appStatus = { kind: 'error', message };
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

  const skeletonBarClass = "rounded bg-gray-200 dark:bg-[#333] animate-pulse";
  const skeletonRowClass = "flex gap-3 px-3";
</script>

{#if appStatus.kind === 'loading'}
  <!-- Loading skeleton matching two-pane layout -->
  <main class={cn("flex h-full w-full flex-col bg-[#ececec] dark:bg-[#1e1e1e]")}>
    <!-- Header bar skeleton -->
    <div class={cn("flex items-center justify-between border-b border-gray-300 dark:border-[#3a3a3a] px-3 py-2")}>
      <div class={cn(skeletonBarClass, "h-4 w-24")}></div>
      <div class={cn("flex gap-2")}>
        <div class={cn(skeletonBarClass, "h-5 w-20")}></div>
        <div class={cn(skeletonBarClass, "h-5 w-20")}></div>
      </div>
    </div>

    <!-- Two-pane content skeleton -->
    <div class={cn("flex flex-1 min-h-0")}>
      <!-- Left pane (Fans) -->
      <div class={cn("flex w-1/2 flex-col border-r border-gray-300 dark:border-[#3a3a3a] bg-white dark:bg-[#1e1e1e]")}>
        <div class={cn("border-b border-gray-300 dark:border-[#3a3a3a] px-3 py-1.5")}>
          <div class={cn(skeletonBarClass, "h-3 w-12")}></div>
        </div>
        <div class={cn("flex flex-col gap-3 p-3")}>
          <div class={cn(skeletonRowClass)}>
            <div class={cn(skeletonBarClass, "h-3 w-32")}></div>
            <div class={cn(skeletonBarClass, "h-3 w-16")}></div>
          </div>
          <div class={cn(skeletonRowClass)}>
            <div class={cn(skeletonBarClass, "h-3 w-28")}></div>
            <div class={cn(skeletonBarClass, "h-3 w-16")}></div>
          </div>
          <div class={cn(skeletonRowClass)}>
            <div class={cn(skeletonBarClass, "h-3 w-36")}></div>
            <div class={cn(skeletonBarClass, "h-3 w-16")}></div>
          </div>
        </div>
      </div>

      <!-- Right pane (Sensors) -->
      <div class={cn("flex w-1/2 flex-col bg-white dark:bg-[#1e1e1e]")}>
        <div class={cn("border-b border-gray-300 dark:border-[#3a3a3a] px-3 py-1.5")}>
          <div class={cn(skeletonBarClass, "h-3 w-16")}></div>
        </div>
        <div class={cn("flex flex-col gap-3 p-3")}>
          <div class={cn(skeletonRowClass)}>
            <div class={cn(skeletonBarClass, "h-3 w-40")}></div>
            <div class={cn(skeletonBarClass, "h-3 w-12")}></div>
          </div>
          <div class={cn(skeletonRowClass)}>
            <div class={cn(skeletonBarClass, "h-3 w-36")}></div>
            <div class={cn(skeletonBarClass, "h-3 w-12")}></div>
          </div>
          <div class={cn(skeletonRowClass)}>
            <div class={cn(skeletonBarClass, "h-3 w-44")}></div>
            <div class={cn(skeletonBarClass, "h-3 w-12")}></div>
          </div>
          <div class={cn(skeletonRowClass)}>
            <div class={cn(skeletonBarClass, "h-3 w-32")}></div>
            <div class={cn(skeletonBarClass, "h-3 w-12")}></div>
          </div>
          <div class={cn(skeletonRowClass)}>
            <div class={cn(skeletonBarClass, "h-3 w-38")}></div>
            <div class={cn(skeletonBarClass, "h-3 w-12")}></div>
          </div>
        </div>
      </div>
    </div>

    <!-- Footer skeleton -->
    <div class={cn("flex items-center justify-between border-t border-gray-300 dark:border-[#3a3a3a] px-3 py-1.5")}>
      <div class={cn(skeletonBarClass, "h-3 w-32")}></div>
      <div class={cn(skeletonBarClass, "h-3 w-20")}></div>
    </div>
  </main>
{:else if appStatus.kind === 'error'}
  <!-- Error state -->
  <main class={cn("flex h-full w-full items-center justify-center bg-[#ececec] dark:bg-[#1e1e1e]")}>
    <div class={cn("flex flex-col items-center gap-1")}>
      <p class={cn("text-[12px] font-medium text-(--text-primary)")}>Failed to connect to sensor backend</p>
      <p class={cn("text-[11px] text-(--text-muted)")}>{appStatus.message}</p>
    </div>
  </main>
{:else}
  <!-- Ready state -->
  <main class={cn("flex h-full w-full")}>
    <DesktopDashboard {fans} {sensorData} />
  </main>
{/if}
