<script lang="ts">
import { AlertTriangle } from "lucide-svelte";
import { untrack } from "svelte";
import { fade, scale } from "svelte/transition";
import { cn } from "$lib/cn";
import { getFanControlSensors } from "$lib/sensorListPaneState";
import {
	installHelper,
	reconnectWriter,
	setFanConstantRpm,
	setFanSensorControl,
} from "$lib/tauriCommands";
import type { FanControlConfig, FanData, Sensor } from "$lib/types";
import {
	backdropSurface,
	buttonPrimary,
	buttonSecondary,
	inputClass,
	sheetSurface,
} from "$lib/ui";

interface Props {
	fan: FanData;
	sensors: Sensor[];
	currentConfig?: FanControlConfig;
	onclose: () => void;
}

const { fan, sensors, currentConfig, onclose }: Props = $props();

const tempSensors = $derived(getFanControlSensors(sensors));

type ControlMode = "constant_rpm" | "sensor_based";

let selectedMode: ControlMode = $state(
	untrack(() =>
		currentConfig?.mode === "sensor_based"
			? "sensor_based"
			: "constant_rpm"
	)
);
let constantRpm: number = $state(
	untrack(() =>
		currentConfig?.mode === "constant_rpm"
			? currentConfig.target_rpm
			: fan.mode === "forced"
				? Math.round(fan.target)
				: Math.round(fan.min + (fan.max - fan.min) / 2)
	)
);
let selectedSensorKey: string = $state(
	untrack(() =>
		currentConfig?.mode === "sensor_based"
			? currentConfig.sensor_key
			: getFanControlSensors(sensors)[0]?.key ?? ""
	)
);
let tempLow: number = $state(
	untrack(() => currentConfig?.mode === "sensor_based" ? currentConfig.temp_low : 40)
);
let tempHigh: number = $state(
	untrack(() => currentConfig?.mode === "sensor_based" ? currentConfig.temp_high : 85)
);
let isSubmitting: boolean = $state(false);
let errorMessage: string = $state("");
let extremeWarningAcknowledged: boolean = $state(false);

let dialogEl: HTMLDivElement | undefined = $state(undefined);
let okButtonEl: HTMLButtonElement | undefined = $state(undefined);

$effect(() => {
	okButtonEl?.focus();
});

const clampedRpm = $derived(
	Math.max(fan.min, Math.min(fan.max, Math.round(constantRpm))),
);

const selectedSensorValue = $derived(
	tempSensors.find((s) => s.key === selectedSensorKey)?.value ?? null,
);

const tempPosition = $derived(
	selectedSensorValue !== null && tempHigh > tempLow
		? Math.max(0, Math.min(1, (selectedSensorValue - tempLow) / (tempHigh - tempLow)))
		: null,
);

const rangeBarAriaLabel = $derived(
	selectedSensorValue !== null
		? `Fan speed range: minimum at ${tempLow}°C, maximum at ${tempHigh}°C. Current sensor temperature is ${selectedSensorValue}°C.`
		: `Fan speed range: minimum at ${tempLow}°C, maximum at ${tempHigh}°C.`,
);

async function handleSubmit(): Promise<void> {
	isSubmitting = true;
	errorMessage = "";

	try {
		if (selectedMode === "constant_rpm") {
			await setFanConstantRpm(fan.index, clampedRpm);
		} else {
			await setFanSensorControl(
				fan.index,
				selectedSensorKey,
				tempLow,
				tempHigh,
			);
		}
		onclose();
	} catch (error) {
		errorMessage = error instanceof Error ? error.message : String(error);
	} finally {
		isSubmitting = false;
	}
}

const isPrivilegeError = $derived(
	errorMessage.includes("root") ||
		errorMessage.includes("privileges") ||
		errorMessage.includes("not available") ||
		errorMessage.includes("Unknown key") ||
		errorMessage.includes("privileged helper")
);

async function handleGrantAccess(): Promise<void> {
	try {
		await installHelper();
		await reconnectWriter();
		errorMessage = "";
		await handleSubmit();
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		if (!msg.includes("cancelled") && !msg.includes("canceled")) {
			errorMessage = msg;
		}
	}
}

function handleCancel(): void {
	onclose();
}

function handleKeydown(event: KeyboardEvent): void {
	if (event.key === "Escape") {
		onclose();
		return;
	}
	if (event.key === "Tab" && dialogEl) {
		const focusableSelector =
			'button:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';
		const focusableElements = Array.from(
			dialogEl.querySelectorAll<HTMLElement>(focusableSelector),
		);
		const firstFocusable = focusableElements.at(0);
		const lastFocusable = focusableElements.at(-1);
		if (event.shiftKey && document.activeElement === firstFocusable) {
			event.preventDefault();
			lastFocusable?.focus();
		} else if (!event.shiftKey && document.activeElement === lastFocusable) {
			event.preventDefault();
			firstFocusable?.focus();
		}
	}
}

const rpmRange = $derived(fan.max - fan.min);
const isExtremeRpm = $derived(
	selectedMode === "constant_rpm" &&
	(clampedRpm < fan.min + rpmRange * 0.2 || clampedRpm > fan.min + rpmRange * 0.8)
);
const isExtremeTemp = $derived(
	selectedMode === "sensor_based" &&
	(tempHigh > 100 || tempLow < 25)
);
const hasExtremeSettings = $derived(isExtremeRpm || isExtremeTemp);
const needsConfirmation = $derived(hasExtremeSettings && !extremeWarningAcknowledged);

const extremeWarningMessage = $derived(
	isExtremeRpm
		? clampedRpm < fan.min + rpmRange * 0.2
			? "Very low fan speed may cause overheating. The system will intervene at 95°C."
			: "Very high fan speed increases noise and wear on the fan bearings."
		: isExtremeTemp
			? tempHigh > 100
				? "Maximum temperature above 100°C risks thermal damage to components."
				: "Low starting temperature (< 25°C) will keep fans running constantly."
			: ""
);
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class={cn('fixed inset-0 z-50 flex items-center justify-center')}
  transition:fade={{ duration: 150 }}
>
  <button
    type="button"
    class={cn(backdropSurface)}
    onclick={handleCancel}
    aria-label="Close dialog"
    tabindex="-1"
  ></button>

  <div
    bind:this={dialogEl}
    class={cn(sheetSurface, 'relative w-[420px]')}
    role="dialog"
    aria-modal="true"
    aria-label={`Change fan control for ${fan.label}`}
    aria-describedby="fan-modal-body"
    transition:scale={{ duration: 150, start: 0.96, opacity: 0 }}
  >
    <div class={cn('px-6 pt-5 pb-4 text-center')}>
      <h2 class={cn('text-[15px] font-semibold text-(--text-primary)')}>
        Change fan control for '{fan.label}'
      </h2>
    </div>

    <div id="fan-modal-body" class={cn('px-6 pb-4 space-y-4')}>
      <label class={cn('flex items-center gap-3 cursor-pointer')}>
        <input
          type="radio"
          name="control-mode"
          value="constant_rpm"
          bind:group={selectedMode}
          class="accent-(--text-primary)"
        />
        <span class={cn('text-[13px] text-(--text-primary)')}>Constant RPM value</span>
      </label>

      {#if selectedMode === 'constant_rpm'}
        <div class={cn('ml-7 flex items-center gap-2')}>
          <input
            type="number"
            bind:value={constantRpm}
            min={fan.min}
            max={fan.max}
            step={100}
            aria-label={`Target RPM for ${fan.label}, range ${Math.round(fan.min)} to ${Math.round(fan.max)}`}
            class={cn(inputClass, 'w-24 text-right')}
          />
          <span class={cn('text-[11px] text-(--text-muted)')}>
            RPM ({Math.round(fan.min)} – {Math.round(fan.max)})
          </span>
        </div>
      {/if}

      <div class={cn('border-t border-(--border-subtle)')}></div>

      <label class={cn('flex items-center gap-3 cursor-pointer')}>
        <input
          type="radio"
          name="control-mode"
          value="sensor_based"
          bind:group={selectedMode}
          class="accent-(--text-primary)"
        />
        <span class={cn('text-[13px] text-(--text-primary)')}>Sensor-based value</span>

        {#if selectedMode === 'sensor_based'}
          <select
            bind:value={selectedSensorKey}
            aria-label={`Temperature sensor for ${fan.label}`}
            class={cn(inputClass, 'ml-auto min-w-[180px]')}
          >
            {#each tempSensors as sensor (sensor.key)}
              <option value={sensor.key}>{sensor.name}</option>
            {/each}
          </select>
        {/if}
      </label>

      {#if selectedMode === 'sensor_based'}
        <div class={cn('ml-7 space-y-3')}>
          <div class={cn('flex items-center justify-between')}>
            <span class={cn('text-[12px] text-(--text-secondary) max-w-[240px]')}>
              Temperature that the fan speed will start to increase from:
            </span>
            <div class={cn('flex items-center gap-1')}>
              <input
                type="number"
                bind:value={tempLow}
                min={0}
                max={120}
                step={1}
                aria-label="Temperature to start increasing fan speed, degrees Celsius"
                class={cn(inputClass, 'w-16 text-right')}
              />
              <span class={cn('text-[11px] text-(--text-muted)')}>°C</span>
            </div>
          </div>

          <div class={cn('flex items-center justify-between')}>
            <span class={cn('text-[12px] text-(--text-secondary)')}>Maximum temperature:</span>
            <div class={cn('flex items-center gap-1')}>
              <input
                type="number"
                bind:value={tempHigh}
                min={0}
                max={120}
                step={1}
                aria-label="Maximum temperature threshold, degrees Celsius"
                class={cn(inputClass, 'w-16 text-right')}
              />
              <span class={cn('text-[11px] text-(--text-muted)')}>°C</span>
            </div>
          </div>

          <div class={cn('mt-1 space-y-1')} role="img" aria-label={rangeBarAriaLabel}>
            <div class={cn('relative flex justify-between font-mono text-[10px] tabular-nums text-(--text-muted)')}>
              <span>{tempLow}°C</span>
              {#if tempPosition !== null && selectedSensorValue !== null}
                <span
                  class={cn('absolute font-medium text-(--text-secondary)')}
                  style="left: {tempPosition * 100}%; transform: translateX(-50%)"
                >{Math.round(selectedSensorValue)}°C</span>
              {/if}
              <span>{tempHigh}°C</span>
            </div>
            <div class={cn('relative h-2 rounded-full overflow-hidden bg-(--surface-0)')}>
              <div
                class={cn('absolute inset-0 rounded-full')}
                style="background: linear-gradient(to right, var(--status-normal), var(--status-warm), var(--status-hot))"
              ></div>
              {#if tempPosition !== null}
                <div
                  class={cn('absolute top-1/2 size-3 rounded-full border-2 border-(--surface-2) bg-(--text-primary) shadow-sm')}
                  style="left: {tempPosition * 100}%; transform: translate(-50%, -50%)"
                ></div>
              {/if}
            </div>
            <div class={cn('flex justify-between text-[10px] text-(--text-muted)')}>
              <span>Min fan</span>
              <span>Max fan</span>
            </div>
          </div>
        </div>
      {/if}

      {#if hasExtremeSettings}
        <div class={cn('flex items-start gap-2 rounded-[10px] border border-(--border-subtle) bg-(--surface-0) p-3 space-y-2')} role="alert">
          <AlertTriangle size={14} class={cn('mt-0.5 shrink-0 text-(--text-primary)')} strokeWidth={1.75} />
          <div class={cn('space-y-2')}>
            <p class={cn('text-[11px] font-medium text-(--text-primary)')}>
              {extremeWarningMessage}
            </p>
            {#if !extremeWarningAcknowledged}
              <label class={cn('flex items-center gap-2 cursor-pointer')}>
                <input
                  type="checkbox"
                  bind:checked={extremeWarningAcknowledged}
                  class="accent-(--text-primary)"
                />
                <span class={cn('text-[11px] text-(--text-secondary)')}>I understand the risks</span>
              </label>
            {/if}
          </div>
        </div>
      {/if}

      {#if errorMessage}
        <div
          class={cn('text-[11px] text-center space-y-2')}
          role="alert"
          aria-live="assertive"
        >
          <p class={cn('text-(--text-primary)')}>
            {isPrivilegeError
              ? "Fan control needs a one-time setup after install. Grant access to install the privileged helper."
              : errorMessage}
          </p>
          {#if isPrivilegeError}
            <button
              type="button"
              class={cn(buttonPrimary)}
              onclick={handleGrantAccess}
            >
              Grant Access
            </button>
          {/if}
        </div>
      {/if}
    </div>

    <div class={cn('flex justify-end gap-2 px-6 pb-5')}>
      <button type="button" class={cn(buttonSecondary)} onclick={handleCancel}>
        Cancel
      </button>
      <button
        type="button"
        bind:this={okButtonEl}
        class={cn(buttonPrimary)}
        disabled={isSubmitting || needsConfirmation}
        onclick={handleSubmit}
      >
        OK
      </button>
    </div>
  </div>
</div>
