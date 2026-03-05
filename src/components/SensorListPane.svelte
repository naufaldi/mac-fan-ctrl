<script lang="ts">
  import { getSensors, listenToSensorUpdates } from "$lib/tauriCommands";
  import {
    getDetailSensorsInDisplayOrder,
    getReadMoreLabel,
    getSummarySensorsForDisplay,
    shouldShowReadMore,
  } from "$lib/sensorListPaneState";
  import type { SensorData as DesignTokenSensorData } from "$lib/designTokens";
  import type { Sensor, SensorData as FullSensorData } from "$lib/types";
  import { cn } from "$lib/cn";

  interface Props {
    sensors?: DesignTokenSensorData[];
  }

  let { sensors }: Props = $props();

  function convertSensors(inputSensors: DesignTokenSensorData[] | undefined): FullSensorData | null {
    if (!inputSensors || inputSensors.length === 0) {
      return null;
    }

    const details: Sensor[] = inputSensors
      .filter((sensor): sensor is DesignTokenSensorData & { value: number } => typeof sensor.value === "number")
      .map((sensor) => ({
        key: sensor.id,
        name: sensor.label,
        value: sensor.value,
        unit: "C",
        sensor_type: sensor.status === "normal" ? "Cpu" : "Other",
      }));

    const cpu_package = details.find((sensor) => sensor.name.toLowerCase().includes("cpu")) || null;
    const gpu = details.find((sensor) => sensor.name.toLowerCase().includes("gpu")) || null;
    const ram = details.find((sensor) => sensor.name.toLowerCase().includes("ram") || sensor.name.toLowerCase().includes("memory")) || null;
    const ssd = details.find((sensor) => sensor.name.toLowerCase().includes("ssd") || sensor.name.toLowerCase().includes("storage")) || null;

    return {
      summary: { cpu_package, gpu, ram, ssd },
      details,
    };
  }

  let sensorData: FullSensorData | null = $state(null);
  let expanded = $state(false);
  let loading = $state(true);
  let error: string | null = $state(null);
  const summarySensors = $derived(sensorData ? getSummarySensorsForDisplay(sensorData) : []);
  const detailSensors = $derived(sensorData ? getDetailSensorsInDisplayOrder(sensorData) : []);

  $effect(() => {
    if (sensors && sensors.length > 0) {
      sensorData = convertSensors(sensors);
      loading = false;
      error = null;
      return;
    }

    let cancelled = false;
    let unlisten: (() => void) | null = null;

    const loadSensors = async (): Promise<void> => {
      try {
        loading = true;
        error = null;
        sensorData = await getSensors();
      } catch (e) {
        error = e instanceof Error ? e.message : "Failed to load sensors";
      } finally {
        loading = false;
      }

      try {
        unlisten = await listenToSensorUpdates((nextData) => {
          sensorData = nextData;
          error = null;
          loading = false;
        });

        if (cancelled && unlisten) {
          unlisten();
          unlisten = null;
        }
      } catch (e) {
        if (!error) {
          error = e instanceof Error ? e.message : "Failed to subscribe to sensor updates";
        }
      }
    };

    void loadSensors();

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };
  });

  function getStatusColor(value: number | null): string {
    if (value === null) return "text-(--text-muted)";
    if (value > 80) return "text-red-400";
    if (value > 60) return "text-yellow-400";
    return "text-green-400";
  }

  function formatValue(sensor: Sensor): string {
    if (sensor.value === null) {
      return "N/A";
    }

    if (sensor.unit === "W") {
      return `${Math.round(sensor.value)} ${sensor.unit}`;
    }

    return `${Math.round(sensor.value)}°${sensor.unit}`;
  }
</script>

<aside class={cn("min-h-0 overflow-y-auto p-3")} aria-label="Temperature sensors panel">
  <!-- Header -->
  <div
    class={cn("grid grid-cols-[1fr_auto] items-center gap-2 border-b border-(--border-subtle) bg-(--surface-2) px-2 py-1 font-semibold text-(--text-secondary)")}
    role="row"
  >
    <span>Sensor</span>
    <span class={cn("text-right")}>Value</span>
  </div>

  {#if loading}
    <div class={cn("rounded-md bg-(--surface-2) px-2 py-2 text-(--text-muted)")}>
      <span>Loading sensors...</span>
    </div>
  {:else if error}
    <div class={cn("rounded-md bg-red-900/20 px-2 py-2 text-red-400")}>
      <span>{error}</span>
    </div>
  {:else if sensorData}
    <!-- Summary View (Always Visible) -->
    {#if summarySensors.length > 0}
      <div class={cn("grid gap-1")}>
        {#each summarySensors as sensor, index (sensor.key)}
          <div
            class={cn(
              "grid grid-cols-[1fr_auto] items-center gap-2 rounded-md border border-transparent px-2 py-1 hover:border-(--border-subtle) hover:bg-(--surface-hover)",
              index % 2 === 0 ? "bg-(--surface-1)" : "bg-(--surface-2)",
            )}
            role="row"
          >
            <span class={cn("truncate text-(--text-primary)")}>{sensor.name}</span>
            <span class={cn("text-right font-mono [font-variant-numeric:tabular-nums]", getStatusColor(sensor.value))}>
              {formatValue(sensor)}
            </span>
          </div>
        {/each}
      </div>

      <!-- Read More Toggle -->
      {#if shouldShowReadMore(detailSensors.length)}
        <button
          onclick={() => expanded = !expanded}
          class={cn(
            "mt-2 w-full rounded-md border border-transparent bg-(--surface-2) px-2 py-1 text-center font-semibold text-(--text-secondary) transition-colors hover:border-(--border-subtle) hover:bg-(--surface-hover)"
          )}
        >
          {getReadMoreLabel(expanded, detailSensors.length)}
        </button>
      {/if}

      <!-- Detailed View (Expandable) -->
      {#if expanded}
        <div
          class={cn("mt-2 pt-2 border-t border-(--border-subtle)")}
        >
          <div
            class={cn(
              "mb-2 border-b border-(--border-subtle) bg-(--surface-2) px-2 py-1 font-semibold text-(--text-secondary)"
            )}
          >
            All Sensors
          </div>
          <div class={cn("grid gap-1")}>
            {#each detailSensors as sensor (sensor.key)}
              <div
                class={cn("grid grid-cols-[1fr_auto] items-center gap-2 rounded-md border border-transparent px-2 py-1 hover:border-(--border-subtle) hover:bg-(--surface-hover)")}
                role="row"
              >
                <span class={cn("truncate text-(--text-secondary)")}>{sensor.name}</span>
                <span class={cn("text-right font-mono text-(--text-value) [font-variant-numeric:tabular-nums]", getStatusColor(sensor.value))}>
                  {formatValue(sensor)}
                </span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    {:else}
      <div class={cn("rounded-md bg-(--surface-2) px-2 py-2 text-(--text-muted)")}>
        <span>No sensors available.</span>
      </div>
    {/if}
  {:else}
    <div class={cn("rounded-md bg-(--surface-2) px-2 py-2 text-(--text-muted)")}>
      <span>No sensors available.</span>
    </div>
  {/if}
</aside>
