<script lang="ts">
import { AlertCircle, CheckCircle2, Info } from "lucide-svelte";
import { cn } from "$lib/cn";
import {
	buildRpmScale,
	clampRpmTarget,
	type GlobalFanControlMode,
	getManualControlConstraint,
	toGlobalFanControlState,
} from "$lib/globalFanControl";
import {
	requestPrivilegeRestart,
	restoreAllFansAuto,
	setAllFansConstantRpm,
} from "$lib/tauriCommands";
import type { FanData } from "$lib/types";
import { buttonSecondary, segmentedActive, segmentedInactive } from "$lib/ui";

interface Props {
	rawFans: FanData[];
	hasWriteAccess: boolean;
	maxTemperatureC: number | null;
}

const { rawFans, hasWriteAccess, maxTemperatureC }: Props = $props();

type ControlInteraction =
	| { readonly kind: "auto" }
	| { readonly kind: "manual-draft"; readonly targetRpm: number }
	| { readonly kind: "settling"; readonly targetRpm: number }
	| { readonly kind: "applying"; readonly targetRpm: number }
	| { readonly kind: "applied"; readonly targetRpm: number }
	| { readonly kind: "restoring" };

const createAutoInteraction = (): ControlInteraction => ({ kind: "auto" });
let interaction: ControlInteraction = $state(createAutoInteraction());
let countdownMs: number = $state(2000);
let hasHydratedHardwareMode: boolean = $state(false);
let statusMessage: string | null = $state(null);
let statusKind: "info" | "error" | "success" = $state("info");

const globalState = $derived(toGlobalFanControlState(rawFans));
const manualConstraint = $derived(
	getManualControlConstraint(rawFans, globalState.range, maxTemperatureC),
);
const minRpm = $derived(
	manualConstraint.kind === "ready"
		? manualConstraint.minRpm
		: globalState.range.kind === "ready"
			? globalState.range.minRpm
			: 0,
);
const maxRpm = $derived(
	manualConstraint.kind === "ready"
		? manualConstraint.maxRpm
		: globalState.range.kind === "ready"
			? globalState.range.maxRpm
			: 0,
);
const effectiveRange = $derived(
	manualConstraint.kind === "ready"
		? {
				kind: "ready" as const,
				minRpm: manualConstraint.minRpm,
				maxRpm: manualConstraint.maxRpm,
			}
		: globalState.range,
);
const modeForInteraction = (value: ControlInteraction): GlobalFanControlMode =>
	value.kind === "auto" || value.kind === "restoring" ? "auto" : "manual";
const targetForInteraction = (
	value: ControlInteraction,
	fallback: number,
): number => ("targetRpm" in value ? value.targetRpm : fallback);
const isOperationPending = (value: ControlInteraction): boolean =>
	value.kind === "applying" || value.kind === "restoring";
const selectedMode: GlobalFanControlMode = $derived(
	modeForInteraction(interaction),
);
const draftTargetRpm = $derived(
	targetForInteraction(interaction, globalState.targetRpm),
);
const formatRpm = (value: number): string =>
	new Intl.NumberFormat("en-US").format(Math.round(value));
const currentRpmLabel = $derived(
	globalState.currentRpm === null
		? "N/A"
		: `${formatRpm(globalState.currentRpm)} RPM`,
);
const targetValue = $derived(clampRpmTarget(draftTargetRpm, effectiveRange));
const targetPercent = $derived(
	effectiveRange.kind === "ready"
		? ((targetValue - effectiveRange.minRpm) /
				(effectiveRange.maxRpm - effectiveRange.minRpm)) *
				100
		: 0,
);
const targetLabelPercent = $derived(Math.max(7, Math.min(93, targetPercent)));
const rpmScale = $derived(
	effectiveRange.kind === "ready" ? buildRpmScale(minRpm, maxRpm) : [],
);
const operationPending = $derived(isOperationPending(interaction));
const canRestoreAuto = $derived(
	hasWriteAccess && globalState.fanCount > 0 && !operationPending,
);
const operationLabel = $derived.by(() => {
	switch (interaction.kind) {
		case "settling":
			return `Applying in ${(countdownMs / 1000).toFixed(1)}s`;
		case "applying":
			return "Verifying…";
		case "applied":
			return `Verified at ${formatRpm(interaction.targetRpm)} RPM`;
		case "restoring":
			return "Restoring Auto…";
		case "auto":
		case "manual-draft":
			return null;
	}
});

$effect(() => {
	if (!hasHydratedHardwareMode && globalState.fanCount > 0) {
		interaction =
			globalState.mode === "manual"
				? { kind: "applied", targetRpm: globalState.targetRpm }
				: { kind: "auto" };
		hasHydratedHardwareMode = true;
	}
});

$effect(() => {
	const currentInteraction = interaction;
	if (currentInteraction.kind !== "settling") {
		return;
	}

	const startedAt = Date.now();
	countdownMs = 2000;
	const countdownInterval = window.setInterval(() => {
		countdownMs = Math.max(0, 2000 - (Date.now() - startedAt));
	}, 100);
	const applyTimeout = window.setTimeout(() => {
		void applyManualTarget(currentInteraction.targetRpm);
	}, 2000);

	return () => {
		window.clearInterval(countdownInterval);
		window.clearTimeout(applyTimeout);
	};
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

async function applyManualTarget(rpm: number): Promise<void> {
	interaction = { kind: "applying", targetRpm: rpm };
	statusMessage = null;
	try {
		await setAllFansConstantRpm(rpm);
		interaction = { kind: "applied", targetRpm: rpm };
		statusKind = "success";
		statusMessage = `Applied ${rpm} RPM to ${globalState.fanCount} fan${globalState.fanCount === 1 ? "" : "s"}.`;
	} catch (error) {
		interaction = { kind: "auto" };
		statusKind = "error";
		statusMessage = isPrivilegeError(error)
			? "Fan control requires elevated privileges."
			: error instanceof Error
				? error.message
				: String(error);
	}
}

async function handleRestoreAuto(): Promise<void> {
	if (!canRestoreAuto) {
		return;
	}

	interaction = { kind: "restoring" };
	statusMessage = null;
	try {
		await restoreAllFansAuto();
		interaction = { kind: "auto" };
		statusKind = "success";
		statusMessage = `Restored automatic control for ${globalState.fanCount} fan${globalState.fanCount === 1 ? "" : "s"}.`;
	} catch (error) {
		interaction = { kind: "auto" };
		statusKind = "error";
		statusMessage = error instanceof Error ? error.message : String(error);
	}
}

function handleManualMode(): void {
	if (
		!hasWriteAccess ||
		manualConstraint.kind !== "ready" ||
		operationPending
	) {
		return;
	}

	interaction = {
		kind: "manual-draft",
		targetRpm: clampRpmTarget(globalState.targetRpm, effectiveRange),
	};
	statusMessage = null;
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
	interaction = {
		kind: "settling",
		targetRpm: clampRpmTarget(Number(input.value), effectiveRange),
	};
	statusMessage = null;
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
          disabled={!canRestoreAuto}
          onclick={handleRestoreAuto}
        >
          Auto
        </button>
        <button
          type="button"
          class={cn(selectedMode === "manual" ? segmentedActive : segmentedInactive, "min-h-7 text-[13px]")}
          aria-pressed={selectedMode === "manual"}
          disabled={!hasWriteAccess || manualConstraint.kind !== "ready" || operationPending}
          onclick={handleManualMode}
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
        <div class={cn("relative min-w-0 grow pb-9 pt-5")}>
          <span
            class={cn("absolute top-0 -translate-x-1/2 text-[13px] font-medium tabular-nums text-(--accent-blue)")}
			style={`left: ${targetLabelPercent}%`}
          >
            {manualConstraint.kind === "ready" ? `${targetValue} RPM` : "--"}
          </span>
          <input
            type="range"
            min={minRpm}
            max={maxRpm}
            step="50"
            value={targetValue}
            aria-label="Target RPM"
            disabled={manualConstraint.kind !== "ready" || selectedMode === "auto" || !hasWriteAccess || operationPending}
            class={cn("liquid-range w-full")}
            style={`--range-progress: ${targetPercent}%`}
            oninput={handleTargetInput}
          />
          <div class={cn("pointer-events-none absolute inset-x-0 top-7 h-7")} aria-hidden="true">
            {#each rpmScale as tick (tick.rpm)}
              <span
                data-rpm-tick={tick.kind}
                class={cn("absolute top-0 -translate-x-1/2 text-center")}
                style={`left: ${tick.percent}%`}
              >
                <span class={cn("mx-auto block w-px bg-(--border-strong)", tick.kind === "major" ? "h-2" : "h-1")}></span>
                {#if tick.kind === "major"}
                  <span class={cn("mt-0.5 block text-[9px] tabular-nums text-(--text-muted)")}>{tick.rpm}</span>
                {/if}
              </span>
            {/each}
          </div>
        </div>
        <span class={cn("w-10 text-[12px] tabular-nums text-(--text-primary)")}>
          {globalState.range.kind === "ready" ? maxRpm : "--"}
        </span>
        </div>
        <div class={cn("text-center text-[12px] tabular-nums text-(--text-secondary)")}>
          Current average: {currentRpmLabel}
        </div>
      </div>
      <div class={cn("flex min-w-[160px] items-center justify-end text-[12px] tabular-nums text-(--text-secondary)")} role="status">
        {operationLabel ?? (selectedMode === "manual" ? "Adjust RPM to schedule" : "Automatic control")}
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

    {#if manualConstraint.reason}
      <p class={cn("mt-2 text-[11px]", manualConstraint.kind === "blocked" ? "text-(--status-hot)" : "text-(--status-warm)")} role="status">
        {manualConstraint.reason}
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
