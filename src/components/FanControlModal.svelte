<script lang="ts">
import { untrack } from "svelte";
import { fade, scale } from "svelte/transition";
import { cn } from "$lib/cn";
import { getFanControlSensors } from "$lib/sensorListPaneState";
import { requestPrivilegeRestart, setFanConstantRpm, setFanSensorControl } from "$lib/tauriCommands";
import type { FanControlConfig, FanData, Sensor } from "$lib/types";

interface Props {
	fan: FanData;
	sensors: Sensor[];
	currentConfig?: FanControlConfig;
	onclose: () => void;
}

const { fan, sensors, currentConfig, onclose }: Props = $props();

// Filter to sensors relevant for fan control decisions
const tempSensors = $derived(getFanControlSensors(sensors));

// ── Modal state ──────────────────────────────────────────────────────────

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

// ── Element refs ─────────────────────────────────────────────────────────

let dialogEl: HTMLDivElement | undefined = $state(undefined);
let okButtonEl: HTMLButtonElement | undefined = $state(undefined);

// Auto-focus OK button on mount
$effect(() => {
	okButtonEl?.focus();
});

// Clamp RPM to valid range
const clampedRpm = $derived(
	Math.max(fan.min, Math.min(fan.max, Math.round(constantRpm))),
);

// ── Temperature range bar state ──────────────────────────────────────────

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

// ── Handlers ─────────────────────────────────────────────────────────────

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
	errorMessage.includes('root') || errorMessage.includes('privileges') || errorMessage.includes('not available')
);

const isDevModeError = $derived(
	errorMessage.includes('development mode') || errorMessage.includes('sudo pnpm')
);

async function handleRestartWithPrivileges(): Promise<void> {
	try {
		await requestPrivilegeRestart();
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		if (!msg.includes('cancelled') && !msg.includes('canceled')) {
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

// ── Shared styles ────────────────────────────────────────────────────────

const buttonBase =
	"cursor-pointer rounded-[5px] border px-4 py-1.5 text-[12px] font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500";
const cancelButton = cn(
	buttonBase,
	"border-gray-300 dark:border-[#4a4a4a] bg-white dark:bg-[#3a3a3a] text-(--text-primary) shadow-[0_1px_2px_rgba(0,0,0,0.05)] hover:bg-gray-50 dark:hover:bg-[#444]",
);
const okButton = cn(
	buttonBase,
	"border-blue-500 bg-blue-500 text-white shadow-[0_1px_2px_rgba(0,0,0,0.1)] hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed",
);

const inputBase =
	"rounded-[4px] border border-gray-300 dark:border-[#4a4a4a] bg-white dark:bg-[#2a2a2a] px-2 py-1 text-[12px] text-(--text-primary) [font-variant-numeric:tabular-nums] focus:outline-none focus:ring-2 focus:ring-blue-500";
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Overlay wrapper -->
<div
  class={cn('fixed inset-0 z-50 flex items-center justify-center')}
  transition:fade={{ duration: 150 }}
>
  <!-- Backdrop (accessible button, not in tab order) -->
  <button
    type="button"
    class={cn('absolute inset-0 bg-black/30 backdrop-blur-[1px] cursor-default')}
    onclick={handleCancel}
    aria-label="Close dialog"
    tabindex="-1"
  ></button>

  <!-- Modal -->
  <div
    bind:this={dialogEl}
    class={cn(
      'relative w-[420px] rounded-lg border border-gray-300 dark:border-[#4a4a4a] bg-[#ececec] dark:bg-[#2d2d2d] shadow-2xl'
    )}
    role="dialog"
    aria-modal="true"
    aria-label={`Change fan control for ${fan.label}`}
    aria-describedby="fan-modal-body"
    transition:scale={{ duration: 150, start: 0.95, opacity: 0 }}
  >
    <!-- Title -->
    <div class={cn('px-6 pt-5 pb-4 text-center')}>
      <h2 class={cn('text-[13px] font-semibold text-(--text-primary)')}>
        Change fan control for '{fan.label}'
      </h2>
    </div>

    <!-- Body -->
    <div id="fan-modal-body" class={cn('px-6 pb-4 space-y-4')}>
      <!-- Constant RPM option -->
      <label class={cn('flex items-center gap-3 cursor-pointer')}>
        <input
          type="radio"
          name="control-mode"
          value="constant_rpm"
          bind:group={selectedMode}
          class="accent-blue-500"
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
            class={cn(inputBase, 'w-24 text-right')}
          />
          <span class={cn('text-[11px] text-(--text-muted)')}>
            RPM ({Math.round(fan.min)} – {Math.round(fan.max)})
          </span>
        </div>
      {/if}

      <!-- Separator -->
      <div class={cn('border-t border-gray-300 dark:border-[#4a4a4a]')}></div>

      <!-- Sensor-based option -->
      <label class={cn('flex items-center gap-3 cursor-pointer')}>
        <input
          type="radio"
          name="control-mode"
          value="sensor_based"
          bind:group={selectedMode}
          class="accent-blue-500"
        />
        <span class={cn('text-[13px] text-(--text-primary)')}>Sensor-based value</span>

        {#if selectedMode === 'sensor_based'}
          <select
            bind:value={selectedSensorKey}
            aria-label={`Temperature sensor for ${fan.label}`}
            class={cn(inputBase, 'ml-auto min-w-[180px]')}
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
                class={cn(inputBase, 'w-16 text-right')}
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
                class={cn(inputBase, 'w-16 text-right')}
              />
              <span class={cn('text-[11px] text-(--text-muted)')}>°C</span>
            </div>
          </div>

          <!-- Temperature range bar -->
          <div class={cn('mt-1 space-y-1')} role="img" aria-label={rangeBarAriaLabel}>
            <div class={cn('relative flex justify-between text-[10px] font-mono [font-variant-numeric:tabular-nums] text-(--text-muted)')}>
              <span>{tempLow}°C</span>
              {#if tempPosition !== null && selectedSensorValue !== null}
                <span
                  class={cn('absolute text-(--text-secondary) font-medium')}
                  style="left: {tempPosition * 100}%; transform: translateX(-50%)"
                >{Math.round(selectedSensorValue)}°C</span>
              {/if}
              <span>{tempHigh}°C</span>
            </div>
            <div class={cn('relative h-2 rounded-full overflow-hidden bg-(--surface-muted)')}>
              <div
                class={cn('absolute inset-0 rounded-full')}
                style="background: linear-gradient(to right, var(--status-normal), var(--status-warm), var(--status-hot))"
              ></div>
              {#if tempPosition !== null}
                <div
                  class={cn('absolute top-1/2 w-3 h-3 rounded-full border-2 border-[#ececec] dark:border-[#2d2d2d] bg-(--text-primary) shadow-sm')}
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

      <!-- Error message -->
      {#if errorMessage}
        <div
          class={cn('text-[11px] text-center space-y-2')}
          role="alert"
          aria-live="assertive"
        >
          <p class="text-red-500">{errorMessage}</p>
          {#if isPrivilegeError && !isDevModeError}
            <button
              type="button"
              class={cn(buttonBase, 'min-h-[32px] border-amber-500 bg-amber-500 text-white shadow-[0_1px_2px_rgba(0,0,0,0.1)] hover:bg-amber-600')}
              onclick={handleRestartWithPrivileges}
            >
              Restart with Admin Privileges
            </button>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Footer buttons -->
    <div class={cn('flex justify-end gap-2 px-6 pb-5')}>
      <button type="button" class={cancelButton} onclick={handleCancel}>
        Cancel
      </button>
      <button
        type="button"
        bind:this={okButtonEl}
        class={okButton}
        disabled={isSubmitting}
        onclick={handleSubmit}
      >
        OK
      </button>
    </div>
  </div>
</div>
