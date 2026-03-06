<script lang="ts">
import { untrack } from "svelte";
import { cn } from "$lib/cn";
import { setFanConstantRpm, setFanSensorControl, requestPrivilegeRestart } from "$lib/tauriCommands";
import type { FanData, Sensor } from "$lib/types";

interface Props {
	fan: FanData;
	sensors: Sensor[];
	onclose: () => void;
}

const { fan, sensors, onclose }: Props = $props();

// Filter to temperature sensors only for the dropdown
const tempSensors = $derived(
	sensors.filter((s) => s.unit === "C" && s.value !== null),
);

// ── Modal state ──────────────────────────────────────────────────────────

type ControlMode = "constant_rpm" | "sensor_based";

let selectedMode: ControlMode = $state("constant_rpm");
let constantRpm: number = $state(untrack(() => Math.round(fan.min + (fan.max - fan.min) / 2)));
let selectedSensorKey: string = $state(untrack(() => sensors.filter((s) => s.unit === "C" && s.value !== null)[0]?.key ?? ""));
let tempLow: number = $state(33);
let tempHigh: number = $state(85);
let isSubmitting: boolean = $state(false);
let errorMessage: string = $state("");

// Clamp RPM to valid range
const clampedRpm = $derived(
	Math.max(fan.min, Math.min(fan.max, Math.round(constantRpm))),
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

function handleBackdropClick(event: MouseEvent): void {
	if (event.target === event.currentTarget) {
		onclose();
	}
}

function handleKeydown(event: KeyboardEvent): void {
	if (event.key === "Escape") {
		onclose();
	}
}

// ── Shared styles ────────────────────────────────────────────────────────

const buttonBase =
	"rounded-[5px] border px-4 py-1.5 text-[12px] font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500";
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

<!-- Backdrop -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class={cn('fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-[1px]')}
  onclick={handleBackdropClick}
>
  <!-- Modal -->
  <div
    class={cn(
      'w-[420px] rounded-lg border border-gray-300 dark:border-[#4a4a4a] bg-[#ececec] dark:bg-[#2d2d2d] shadow-2xl'
    )}
    role="dialog"
    aria-modal="true"
    aria-label={`Change fan control for ${fan.label}`}
  >
    <!-- Title -->
    <div class={cn('px-6 pt-5 pb-4 text-center')}>
      <h2 class={cn('text-[13px] font-semibold text-(--text-primary)')}>
        Change fan control for '{fan.label}'
      </h2>
    </div>

    <!-- Body -->
    <div class={cn('px-6 pb-4 space-y-4')}>
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
                class={cn(inputBase, 'w-16 text-right')}
              />
              <span class={cn('text-[11px] text-(--text-muted)')}>°C</span>
            </div>
          </div>
        </div>
      {/if}

      <!-- Error message -->
      {#if errorMessage}
        <div class={cn('text-[11px] text-center space-y-2')}>
          <p class="text-red-500">{errorMessage}</p>
          {#if isPrivilegeError}
            <button
              type="button"
              class={cn(buttonBase, 'border-amber-500 bg-amber-500 text-white shadow-[0_1px_2px_rgba(0,0,0,0.1)] hover:bg-amber-600')}
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
        class={okButton}
        disabled={isSubmitting}
        onclick={handleSubmit}
      >
        OK
      </button>
    </div>
  </div>
</div>
