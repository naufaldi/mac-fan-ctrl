<script lang="ts">
import { Fan as FanIcon } from "lucide-svelte";
import { cn } from "$lib/cn";
import { toFanRows } from "$lib/dashboardLayout";
import type { SensorData } from "$lib/designTokens";
import {
	getFanControlConfigs,
	requestPrivilegeRestart,
	setFanAuto,
} from "$lib/tauriCommands";
import type { FanControlConfig, FanData, Sensor } from "$lib/types";
import FanControlModal from "./FanControlModal.svelte";

interface Props {
	fans: SensorData[];
	rawFans: FanData[];
	sensors: Sensor[];
	hasWriteAccess: boolean;
}

const { fans, rawFans, sensors, hasWriteAccess }: Props = $props();

const fanRows = $derived(toFanRows(fans));
const controlBaseClass =
	"rounded-full text-[11px] px-3 py-0.5 text-center transition-colors focus-visible:outline-none cursor-pointer";
const controlInactiveClass =
	"border border-(--border-subtle) bg-(--surface-1) text-(--text-secondary) hover:bg-(--surface-hover)";
const controlActiveClass =
	"bg-amber-500/20 border border-amber-500/40 text-amber-700 dark:bg-amber-400/15 dark:border-amber-400/30 dark:text-amber-300 font-medium";

// ── Modal state ──────────────────────────────────────────────────────────

let modalFan: FanData | null = $state(null);
let modalConfig: FanControlConfig | undefined = $state(undefined);
let modalTriggerEl: HTMLButtonElement | null = $state(null);
let privilegeError: string | null = $state(null);

let fanConfigs: Record<string, FanControlConfig> = $state({});

$effect(() => {
	getFanControlConfigs()
		.then((configs) => {
			fanConfigs = configs;
		})
		.catch((err) =>
			console.error("[mac-fan-ctrl] Failed to fetch fan configs:", err),
		);
});

function openCustomModal(fanIndex: number, triggerEl: HTMLButtonElement): void {
	const fan = rawFans.find((f) => f.index === fanIndex);
	if (fan) {
		modalFan = fan;
		modalConfig = fanConfigs[String(fan.index)];
		modalTriggerEl = triggerEl;
	}
}

function closeModal(): void {
	const trigger = modalTriggerEl;
	modalFan = null;
	modalTriggerEl = null;
	queueMicrotask(() => trigger?.focus());
}

function isPrivilegeError(error: unknown): boolean {
	const msg = error instanceof Error ? error.message : String(error);
	return (
		msg.includes("root") ||
		msg.includes("privileges") ||
		msg.includes("not available")
	);
}

async function handleAutoClick(fanIndex: number): Promise<void> {
	privilegeError = null;
	try {
		await setFanAuto(fanIndex);
	} catch (error) {
		if (isPrivilegeError(error)) {
			privilegeError = "Fan control requires elevated privileges.";
		} else {
			console.error("[mac-fan-ctrl] Failed to set fan to auto:", error);
		}
	}
}

function isDevModeError(msg: string): boolean {
	return msg.includes("development mode") || msg.includes("sudo pnpm");
}

async function handleRestartWithPrivileges(): Promise<void> {
	try {
		await requestPrivilegeRestart();
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		if (msg.includes("cancelled") || msg.includes("canceled")) {
			privilegeError = null;
		} else if (isDevModeError(msg)) {
			privilegeError = msg;
		} else {
			console.error("[mac-fan-ctrl] Privilege restart failed:", error);
		}
	}
}
</script>

<section class={cn("min-h-0 overflow-y-auto border-r border-(--border-subtle) bg-(--surface-1)")}>
  <!-- Header -->
  <div
    class={cn("sticky top-0 grid grid-cols-[100px_1fr_280px] items-center border-b border-(--border-subtle) bg-(--surface-2) text-[11px] font-medium text-gray-600 dark:text-gray-300")}
    role="row"
  >
    <div class="px-2 py-1 flex items-center border-r border-(--border-subtle)">Fan</div>
    <div class="px-2 py-1 flex items-center border-r border-(--border-subtle)">Min/Current/Max RPM</div>
    <div class="px-2 py-1 flex items-center">Control</div>
  </div>

  {#if privilegeError}
    <div class={cn("flex items-center justify-between gap-2 border-b border-amber-300 dark:border-amber-700 bg-amber-50 dark:bg-amber-900/30 px-3 py-1.5 text-[11px] text-amber-800 dark:text-amber-200")}>
      <span>{privilegeError}</span>
      {#if !isDevModeError(privilegeError)}
        <button
          type="button"
          class={cn("shrink-0 rounded-[4px] border border-amber-400 dark:border-amber-600 bg-amber-100 dark:bg-amber-800/50 px-2 py-0.5 text-[11px] font-medium text-amber-900 dark:text-amber-100 hover:bg-amber-200 dark:hover:bg-amber-700/50 transition-colors")}
          onclick={handleRestartWithPrivileges}
        >
          Restart with Admin Privileges
        </button>
      {/if}
    </div>
  {/if}

  <div class={cn("flex flex-col")}>
    {#if fanRows.length === 0}
      <div class={cn("px-2 py-2 text-(--text-muted)")}>
        <span>No fan telemetry available.</span>
      </div>
    {:else}
      {#each fanRows as fan (fan.id)}
        <div
          class={cn(
            "grid grid-cols-[100px_1fr_280px] items-center odd:bg-(--surface-1) even:bg-(--surface-2) hover:bg-(--surface-hover)"
          )}
          role="row"
        >
          <div class={cn("flex min-w-0 items-center gap-2 px-2 py-1 text-[12px] text-(--text-primary)")}>
            <FanIcon size={14} class="text-(--text-secondary) shrink-0" />
            <span class="truncate">{fan.label}</span>
          </div>
          <div
            class={cn("flex items-baseline gap-1 px-2 py-1 text-[12px] font-mono [font-variant-numeric:tabular-nums]")}
          >
            <span class={cn("text-(--text-muted)")}>{fan.minRpm}</span>
            <span class="text-(--text-muted)">-</span>
            <span class={cn("font-bold text-(--text-value)")}>{fan.currentRpm ?? 'N/A'}</span>
            <span class="text-(--text-muted)">-</span>
            <span class={cn("text-(--text-muted)")}>{fan.maxRpm}</span>
          </div>
          <div class={cn("px-2 py-1 flex items-center justify-center gap-2")}>
            <button
              type="button"
              class={cn(
                controlBaseClass,
                fan.controlMode === 'auto' ? controlActiveClass : controlInactiveClass
              )}
              aria-label={`Set ${fan.label} to auto mode`}
              onclick={() => handleAutoClick(fan.fanIndex)}
            >
              Auto
            </button>
            <button
              type="button"
              class={cn(
                controlBaseClass,
                fan.controlMode === 'constant' ? controlActiveClass : controlInactiveClass
              )}
              aria-label={`Set ${fan.label} to custom mode`}
              onclick={(e: MouseEvent) => openCustomModal(fan.fanIndex, e.currentTarget as HTMLButtonElement)}
            >
              {fan.controlMode === 'constant' ? `Constant value of ${fan.targetRpm}` : 'Custom...'}
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</section>

{#if modalFan}
  <FanControlModal fan={modalFan} {sensors} currentConfig={modalConfig} onclose={closeModal} />
{/if}
