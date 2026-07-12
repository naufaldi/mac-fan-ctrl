<script lang="ts">
import { AlertCircle, CheckCircle2, Info } from "lucide-svelte";
import { cn } from "$lib/cn";
import {
	clampRpmTarget,
	type GlobalFanControlMode,
	toGlobalFanControlState,
} from "$lib/globalFanControl";
import {
	requestPrivilegeRestart,
	restoreAllFansAuto,
	setAllFansConstantRpm,
} from "$lib/tauriCommands";
import type { FanData } from "$lib/types";
import { buttonPrimary, buttonSecondary, segmentedActive, segmentedInactive } from "$lib/ui";

interface Props {
	rawFans: FanData[];
	hasWriteAccess: boolean;
}

const { rawFans, hasWriteAccess }: Props = $props();

const initialControlMode = (): GlobalFanControlMode => "auto";
let selectedMode: GlobalFanControlMode = $state(initialControlMode());
let targetRpm: number = $state(0);
let lastStateSignature: string = $state("");
let isApplying: boolean = $state(false);
let statusMessage: string | null = $state(null);
let statusKind: "info" | "error" | "success" = $state("info");

const globalState = $derived(toGlobalFanControlState(rawFans));
const minRpm = $derived(
	globalState.range.kind === "ready" ? globalState.range.minRpm : 0,
);
const maxRpm = $derived(
	globalState.range.kind === "ready" ? globalState.range.maxRpm : 0,
);
const formatRpm = (value: number): string =>
	new Intl.NumberFormat("en-US").format(Math.round(value));
const currentRpmLabel = $derived(
	globalState.currentRpm === null ? "N/A" : `${formatRpm(globalState.currentRpm)} RPM`,
);
const targetValue = $derived(
	clampRpmTarget(targetRpm, globalState.range),
);
const targetPercent = $derived(
	globalState.range.kind === "ready"
		? ((targetValue - globalState.range.minRpm) /
				(globalState.range.maxRpm - globalState.range.minRpm)) *
			100
		: 0,
);
const targetLabelPercent = $derived(Math.max(7, Math.min(93, targetPercent)));
const canWriteManual = $derived(
	hasWriteAccess &&
		selectedMode === "manual" &&
		globalState.canControl &&
		!isApplying,
);
const canRestoreAuto = $derived(
	hasWriteAccess && globalState.fanCount > 0 && !isApplying,
);

$effect(() => {
	const signature = [
		globalState.mode,
		globalState.targetRpm,
		globalState.fanCount,
		globalState.range.kind,
		globalState.range.kind === "ready" ? globalState.range.minRpm : "none",
		globalState.range.kind === "ready" ? globalState.range.maxRpm : "none",
	].join(":");

	if (signature !== lastStateSignature) {
		selectedMode = globalState.mode;
		targetRpm = globalState.targetRpm;
		lastStateSignature = signature;
	}
});

function isDevModeError(message: string): boolean {
	return message.includes("development mode") || message.includes("sudo pnpm");
}

function isPrivilegeError(error: unknown): boolean {
	const message = error instanceof Error ? error.message : String(error);
	return (
		message.includes("root") ||
		message.includes("privileges") ||
		message.includes("not available") ||
		message.includes("Unknown key") ||
		message.includes("privileged helper")
	);
}

async function handleManualApply(): Promise<void> {
	if (!canWriteManual) {
		return;
	}

	isApplying = true;
	statusMessage = null;
	const rpm = targetValue;
	try {
		await setAllFansConstantRpm(rpm);
		statusKind = "success";
		statusMessage = `Applied ${rpm} RPM to ${globalState.fanCount} fan${globalState.fanCount === 1 ? "" : "s"}.`;
	} catch (error) {
		statusKind = "error";
		statusMessage = isPrivilegeError(error)
			? "Fan control requires elevated privileges."
			: error instanceof Error
				? error.message
				: String(error);
	} finally {
		isApplying = false;
	}
}

async function handleRestoreAuto(): Promise<void> {
	if (!canRestoreAuto) {
		return;
	}

	isApplying = true;
	statusMessage = null;
	try {
		await restoreAllFansAuto();
		selectedMode = "auto";
		statusKind = "success";
		statusMessage = `Restored automatic control for ${globalState.fanCount} fan${globalState.fanCount === 1 ? "" : "s"}.`;
	} catch (error) {
		statusKind = "error";
		statusMessage = error instanceof Error ? error.message : String(error);
	} finally {
		isApplying = false;
	}
}

async function handleRestartWithPrivileges(): Promise<void> {
	try {
		await requestPrivilegeRestart();
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		if (message.includes("cancelled") || message.includes("canceled")) {
			statusMessage = null;
		} else if (isDevModeError(message)) {
			statusKind = "error";
			statusMessage = message;
		} else {
			console.error("[fanguard] Privilege restart failed:", error);
		}
	}
}

function handleTargetInput(event: Event): void {
	const input = event.currentTarget as HTMLInputElement;
	targetRpm = clampRpmTarget(Number(input.value), globalState.range);
}
</script>

<section
  class={cn(
    "bg-(--material-window) px-4 py-3"
  )}
  aria-labelledby="fan-control-heading"
>
  <h2 id="fan-control-heading" class={cn("mb-1.5 text-[12px] font-medium uppercase text-(--text-secondary)")}>
    Fan Control
  </h2>

  <div class={cn("rounded-[7px] border border-(--border-subtle) bg-(--surface-1) px-4 py-3")}>
    <div class={cn("mb-1 grid grid-cols-[110px_minmax(180px,1fr)_auto] items-center gap-4")}>
      <span class={cn("text-[13px] text-(--text-primary)")}>Mode</span>
      <div class={cn("grid h-8 w-[200px] grid-cols-2 rounded-[7px] border border-(--border-subtle) bg-(--material-control) p-0.5")}>
        <button
          type="button"
          class={cn(selectedMode === "auto" ? segmentedActive : segmentedInactive, "min-h-7 text-[13px]")}
          aria-pressed={selectedMode === "auto"}
          onclick={() => { selectedMode = "auto"; }}
        >
          Auto
        </button>
        <button
          type="button"
          class={cn(selectedMode === "manual" ? segmentedActive : segmentedInactive, "min-h-7 text-[13px]")}
          aria-pressed={selectedMode === "manual"}
          onclick={() => { selectedMode = "manual"; }}
        >
          Manual
        </button>
      </div>
      <div></div>
    </div>

    <div class={cn("grid grid-cols-[110px_minmax(260px,1fr)_auto] items-center gap-4")}>
      <span class={cn("text-[13px] text-(--text-primary)")}>Target RPM</span>
      <div class={cn("grid min-w-0 grid-rows-[auto_auto] gap-2")}>
        <div class={cn("flex min-w-0 items-center gap-4")}>
        <span class={cn("w-10 text-right text-[12px] tabular-nums text-(--text-primary)")}>
          {globalState.range.kind === "ready" ? minRpm : "--"}
        </span>
        <div class={cn("relative min-w-0 grow pt-5")}>
          <span
            class={cn("absolute top-0 -translate-x-1/2 text-[13px] font-medium tabular-nums text-(--accent-blue)")}
			style={`left: ${targetLabelPercent}%`}
          >
            {globalState.canControl ? `${targetValue} RPM` : "--"}
          </span>
          <input
            type="range"
            min={minRpm}
            max={maxRpm}
            step="50"
            value={targetValue}
            aria-label="Target RPM"
            disabled={!globalState.canControl || selectedMode === "auto" || !hasWriteAccess || isApplying}
            class={cn("liquid-range w-full")}
            style={`--range-progress: ${targetPercent}%`}
            oninput={handleTargetInput}
          />
        </div>
        <span class={cn("w-10 text-[12px] tabular-nums text-(--text-primary)")}>
          {globalState.range.kind === "ready" ? maxRpm : "--"}
        </span>
        </div>
        <div class={cn("text-center text-[12px] tabular-nums text-(--text-secondary)")}>
          Current average: {currentRpmLabel}
        </div>
      </div>
      <div class={cn("flex items-center justify-end gap-2")}>
        <button
          type="button"
          class={cn(buttonPrimary, "min-h-8 min-w-[96px] rounded-[7px] text-[13px]")}
          disabled={!canWriteManual}
          onclick={handleManualApply}
        >
          {isApplying ? "Applying..." : "Apply"}
        </button>
        <button
          type="button"
          class={cn(buttonSecondary, "min-h-8 min-w-[112px] rounded-[7px] text-[13px]")}
          disabled={!canRestoreAuto}
          onclick={handleRestoreAuto}
        >
          Restore Auto
        </button>
      </div>
    </div>

    <p class={cn("mt-2 flex items-center gap-2 text-[12px] text-(--text-secondary)")}>
      <Info size={16} class={cn("shrink-0")} strokeWidth={1.8} />
      Applies to all fans.
    </p>

    {#if globalState.disabledReason}
      <p class={cn("mt-2 text-[11px] text-(--text-secondary)")} role="status">
        {globalState.disabledReason}
      </p>
    {/if}

    {#if !hasWriteAccess}
      <p class={cn("mt-2 text-[11px] text-(--text-secondary)")} role="status">
        Fan writes need administrator access.
      </p>
    {/if}

    {#if statusMessage}
      <div
        class={cn(
          "mt-2 flex items-center justify-between gap-2 rounded-[8px] border px-2.5 py-1.5 text-[11px]",
          statusKind === "error"
            ? "border-(--status-hot) bg-(--surface-1) text-(--text-primary)"
            : "border-(--border-subtle) bg-(--material-control) text-(--text-secondary)"
        )}
        role={statusKind === "error" ? "alert" : "status"}
      >
        <span class={cn("flex items-center gap-1.5")}>
          {#if statusKind === "error"}
            <AlertCircle size={13} class={cn("text-(--status-hot)")} strokeWidth={1.8} />
          {:else}
            <CheckCircle2 size={13} class={cn("text-(--status-normal)")} strokeWidth={1.8} />
          {/if}
          {statusMessage}
        </span>
        {#if statusKind === "error" && statusMessage === "Fan control requires elevated privileges." && !isDevModeError(statusMessage)}
          <button
            type="button"
            class={cn(buttonSecondary, "shrink-0 px-2 py-0.5 text-[11px]")}
            onclick={handleRestartWithPrivileges}
          >
            Restart with Admin Privileges
          </button>
        {/if}
      </div>
    {/if}
  </div>
</section>
