<script lang="ts">
import type { Component } from "svelte";
import { cn } from "$lib/cn";
import type { SensorData as DesignTokenSensor } from "$lib/designTokens";
import { getSensors, listenToSensorUpdates } from "$lib/tauriCommands";
import type { FanData, SensorData } from "$lib/types";

// Dynamic import prevents DesktopDashboard's import chain from blocking
// the loading skeleton. If any module in the chain throws at runtime,
// the skeleton and error states still render.
const dashboardPromise: Promise<
	Component<{ fans: DesignTokenSensor[]; sensorData: SensorData | null }>
> = import("./components/DesktopDashboard.svelte").then((m) => m.default);

type AppStatus =
	| { readonly kind: "loading" }
	| { readonly kind: "error"; readonly message: string }
	| { readonly kind: "ready" };

let appStatus: AppStatus = $state({ kind: "loading" });
let sensorData: SensorData | null = $state(null);
let fans: DesignTokenSensor[] = $state([]);
let unlisten: (() => void) | null = null;

// Historical data buffer for sparklines (last 60 readings per fan)
const SPARKLINE_BUFFER_SIZE = 60;
const fanHistory: Map<number, number[]> = new Map();

function toDesignToken(fan: FanData, sparklineData?: number[]): DesignTokenSensor {
	return {
		id: `fan-${fan.index}`,
		fanIndex: fan.index,
		label: fan.label,
		value: Math.round(fan.actual),
		unit: "rpm",
		status: "normal",
		minRpm: Math.round(fan.min),
		maxRpm: Math.round(fan.max),
		targetRpm: Math.round(fan.target),
		controlMode: fan.mode === "forced" ? "constant" : "auto",
		sparklineData,
	};
}

function appendToHistory(fanIndex: number, rpm: number): number[] {
	const existing = fanHistory.get(fanIndex) ?? [];
	const updated = [...existing, rpm].slice(-SPARKLINE_BUFFER_SIZE);
	fanHistory.set(fanIndex, updated);
	return updated;
}

function applyUpdate(data: SensorData): void {
	sensorData = data;
	fans = data.fans.map((fan) => {
		const history = appendToHistory(fan.index, Math.round(fan.actual));
		return toDesignToken(fan, history);
	});
	appStatus = { kind: "ready" };
}

$effect(() => {
	let cancelled = false;

	const init = async () => {
		try {
			const initial = await getSensors();
			if (!cancelled) applyUpdate(initial);
		} catch (e) {
			const message = e instanceof Error ? e.message : String(e);
			console.error("[fanguard] Failed to fetch initial sensors:", e);
			if (!cancelled) appStatus = { kind: "error", message };
		}

		try {
			unlisten = await listenToSensorUpdates((data) => {
				if (!cancelled) applyUpdate(data);
			});
			if (cancelled && unlisten) {
				unlisten();
				unlisten = null;
			}
		} catch (e) {
			console.error("[fanguard] Failed to subscribe:", e);
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
  {#await dashboardPromise then DashboardComponent}
    <main class={cn("flex h-full w-full")}>
      <DashboardComponent {fans} {sensorData} />
    </main>
  {:catch error}
    <main class={cn("flex h-full w-full items-center justify-center bg-[#ececec] dark:bg-[#1e1e1e]")}>
      <div class={cn("flex flex-col items-center gap-1")}>
        <p class={cn("text-[12px] font-medium text-(--text-primary)")}>Failed to load dashboard</p>
        <p class={cn("text-[11px] text-(--text-muted)")}>{error instanceof Error ? error.message : String(error)}</p>
      </div>
    </main>
  {/await}
{/if}
