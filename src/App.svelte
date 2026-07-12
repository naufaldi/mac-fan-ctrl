<script lang="ts">
import type { Component } from "svelte";
import { cn } from "$lib/cn";
import type { SensorData as DesignTokenSensor } from "$lib/designTokens";
import { getSensors, listenToSensorUpdates } from "$lib/tauriCommands";
import type { FanData, SensorData } from "$lib/types";

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

const barClass = "rounded-[8px] bg-(--surface-2) animate-pulse";
const rowClass = "flex items-center gap-3 px-4";
</script>

{#if appStatus.kind === 'loading'}
  <main class={cn("flex h-full w-full flex-col bg-(--window-background)")}>
    <div class={cn("flex min-h-14 items-center gap-3 border-b border-(--border-subtle) bg-(--material-toolbar) px-4")}>
      <div class={cn(barClass, "h-3.5 w-20")}></div>
      <div class={cn(barClass, "h-7 w-32")}></div>
      <div class={cn("grow")}></div>
      <div class={cn(barClass, "h-8 w-52")}></div>
      <div class={cn(barClass, "h-8 w-8")}></div>
    </div>

    <div class={cn("border-b border-(--border-subtle) px-4 py-3")}>
      <div class={cn("mb-2 flex items-center justify-between")}>
        <div class={cn(barClass, "h-4 w-24")}></div>
        <div class={cn(barClass, "h-7 w-36")}></div>
      </div>
      <div class={cn(barClass, "h-2 w-full")}></div>
    </div>

    <div class={cn("flex min-h-0 grow flex-col bg-(--surface-1)")}>
      <div class={cn("flex items-center justify-between border-b border-(--border-subtle) px-4 py-2.5")}>
        <div class={cn(barClass, "h-3.5 w-24")}></div>
        <div class={cn(barClass, "h-3 w-4")}></div>
      </div>
      {#each [0, 1] as r}
        <div class={cn(rowClass, "py-2.5", r === 1 && "border-b border-(--border-subtle)")}>
          <div class={cn(barClass, "h-3.5 w-32")}></div>
          <div class={cn(barClass, "ml-auto h-3.5 w-28")}></div>
          <div class={cn(barClass, "h-5 w-20 rounded-full")}></div>
        </div>
      {/each}
      <div class={cn("flex items-center justify-between border-b border-(--border-subtle) px-4 py-2.5")}>
        <div class={cn(barClass, "h-3.5 w-32")}></div>
        <div class={cn(barClass, "h-3 w-4")}></div>
      </div>
      {#each [0, 1, 2] as r}
        <div class={cn(rowClass, "py-1.5", r !== 2 && "border-b border-(--border-subtle)")}>
          <div class={cn(barClass, "h-3 w-40")}></div>
          <div class={cn(barClass, "ml-auto h-3 w-8")}></div>
        </div>
      {/each}
    </div>

    <div class={cn("mt-auto flex min-h-9 items-center gap-4 border-t border-(--border-subtle) bg-(--material-toolbar) px-4")}>
      <div class={cn(barClass, "h-3 w-24")}></div>
      <div class={cn("grow")}></div>
      <div class={cn(barClass, "h-3 w-28")}></div>
    </div>
  </main>
{:else if appStatus.kind === 'error'}
  <main class={cn("flex h-full w-full items-center justify-center bg-(--window-background)")}>
    <div class={cn("flex flex-col items-center gap-1")}>
      <p class={cn("text-[12px] font-medium text-(--text-primary)")}>Failed to connect to sensor backend</p>
      <p class={cn("text-[11px] text-(--text-muted)")}>{appStatus.message}</p>
    </div>
  </main>
{:else}
  {#await dashboardPromise then DashboardComponent}
    <main class={cn("flex h-full w-full bg-(--window-background)")}>
      <DashboardComponent {fans} {sensorData} />
    </main>
  {:catch error}
    <main class={cn("flex h-full w-full items-center justify-center bg-(--window-background)")}>
      <div class={cn("flex flex-col items-center gap-1")}>
        <p class={cn("text-[12px] font-medium text-(--text-primary)")}>Failed to load dashboard</p>
        <p class={cn("text-[11px] text-(--text-muted)")}>{error instanceof Error ? error.message : String(error)}</p>
      </div>
    </main>
  {/await}
{/if}
