<script lang="ts">
  import { getSensors, listenToSensorUpdates } from "$lib/tauriCommands";
  import { getAllSensorsForDisplay } from "$lib/sensorListPaneState";
  import type { SensorData as DesignTokenSensorData } from "$lib/designTokens";
  import type { Sensor, SensorData as FullSensorData } from "$lib/types";
  import { cn } from "$lib/cn";
  import { Cpu, Monitor, HardDrive, Battery, Wifi, Thermometer } from "lucide-svelte";

  interface Props {
    sensors?: DesignTokenSensorData[];
  }

  let { sensors }: Props = $props();

  function convertSensors(inputSensors: DesignTokenSensorData[] | undefined): FullSensorData | null {
    if (!inputSensors || inputSensors.length === 0) {
      return null;
    }

    const details: Sensor[] = inputSensors
      .map((sensor) => ({
        key: sensor.id,
        name: sensor.label,
        value: sensor.value,
        unit: "C",
        sensor_type: sensor.status === "normal" ? "Cpu" : "Other",
        source: "placeholder",
        null_reason: sensor.value === null ? "placeholder" : null,
      }));

    const cpu_package = details.find((sensor) => sensor.name.toLowerCase().includes("cpu")) || null;
    const gpu = details.find((sensor) => sensor.name.toLowerCase().includes("gpu")) || null;
    const ram = details.find((sensor) => sensor.name.toLowerCase().includes("ram") || sensor.name.toLowerCase().includes("memory")) || null;
    const ssd = details.find((sensor) => sensor.name.toLowerCase().includes("ssd") || sensor.name.toLowerCase().includes("storage")) || null;

    return {
      summary: { cpu_package, gpu, ram, ssd },
      details,
      diagnostics: {
        model_id: null,
        diagnostics_enabled: false,
        active_providers: ["design_token"],
        unresolved: [],
      },
    };
  }

  let sensorData: FullSensorData | null = $state(null);
  let loading = $state(true);
  let error: string | null = $state(null);
  const displaySensors = $derived(sensorData ? getAllSensorsForDisplay(sensorData) : []);

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
    if (value > 80) return "text-red-600 dark:text-red-400";
    if (value > 60) return "text-yellow-600 dark:text-yellow-400";
    return "text-gray-900 dark:text-gray-100"; // Changed from green to default text color to match the image
  }

  function formatValue(sensor: Sensor): string {
    if (sensor.name === "Disk Drives:") {
      return "";
    }

    if (sensor.value === null) {
      return "N/A";
    }

    if (sensor.unit === "W") {
      return `${Math.round(sensor.value)} ${sensor.unit}`;
    }

    return `${Math.round(sensor.value)}°${sensor.unit}`;
  }

  function getSensorIcon(name: string) {
    const lowerName = name.toLowerCase();
    if (name === "Disk Drives:") return null;
    if (lowerName.includes('cpu')) return Cpu;
    if (lowerName.includes('gpu')) return Monitor;
    if (lowerName.includes('ram') || lowerName.includes('memory')) return Monitor; // Fallback since MemoryStick isn't always available, let's use Thermometer or something else
    if (lowerName.includes('ssd') || lowerName.includes('storage') || lowerName.includes('hdd')) return HardDrive;
    if (lowerName.includes('battery')) return Battery;
    if (lowerName.includes('airport') || lowerName.includes('wi-fi') || lowerName.includes('wifi')) return Wifi;
    return Thermometer;
  }

  function getSensorIconColor(name: string) {
    const lowerName = name.toLowerCase();
    if (name === "Disk Drives:") return "text-(--text-muted)";
    if (lowerName.includes('cpu')) return "text-green-600 dark:text-green-500";
    if (lowerName.includes('gpu')) return "text-green-600 dark:text-green-500";
    if (lowerName.includes('ram') || lowerName.includes('memory')) return "text-green-600 dark:text-green-500";
    if (lowerName.includes('ssd') || lowerName.includes('storage') || lowerName.includes('hdd')) return "text-gray-600 dark:text-gray-400";
    if (lowerName.includes('battery')) return "text-gray-600 dark:text-gray-400";
    if (lowerName.includes('airport') || lowerName.includes('wi-fi') || lowerName.includes('wifi')) return "text-blue-500";
    return "text-gray-500";
  }
</script>

<aside class={cn("min-h-0 overflow-y-auto bg-(--surface-1)")} aria-label="Temperature sensors panel">
  <!-- Header -->
  <div
    class={cn("sticky top-0 grid grid-cols-[1fr_auto] items-center border-b border-(--border-subtle) bg-(--surface-2) text-[11px] font-medium text-gray-600 dark:text-gray-300")}
    role="row"
  >
    <div class="px-2 py-1 flex items-center border-r border-(--border-subtle)">Sensor</div>
    <div class="px-2 py-1 min-w-[70px] text-right">Value °C</div>
  </div>

  {#if loading}
    <div class={cn("px-2 py-2 text-(--text-muted)")}>
      <span>Loading sensors...</span>
    </div>
  {:else if error}
    <div class={cn("bg-red-900/20 px-2 py-2 text-red-400")}>
      <span>{error}</span>
    </div>
  {:else if displaySensors.length > 0}
    <div class={cn("flex flex-col")}>
      {#each displaySensors as sensor (sensor.key)}
        {@const Icon = getSensorIcon(sensor.name)}
        <div
          class={cn(
            "grid grid-cols-[1fr_auto] items-center odd:bg-(--surface-1) even:bg-(--surface-2) hover:bg-(--surface-hover)"
          )}
          role="row"
        >
          <div class={cn("flex items-center gap-2 truncate px-2 py-1 text-[12px]", sensor.name === "Disk Drives:" ? "text-(--text-muted) font-medium" : "text-(--text-primary)")}>
            {#if Icon}
              <Icon size={14} class={cn("shrink-0", getSensorIconColor(sensor.name))} />
            {/if}
            <span class="truncate">{sensor.name}</span>
          </div>
          <div class={cn("px-2 py-1 min-w-[70px] text-right text-[12px] font-mono [font-variant-numeric:tabular-nums]", getStatusColor(sensor.value))}>
            {formatValue(sensor)}
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class={cn("px-2 py-2 text-(--text-muted)")}>
      <span>No sensors available.</span>
    </div>
  {/if}
</aside>
