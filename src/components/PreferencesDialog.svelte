<script lang="ts">
import { fade, scale } from "svelte/transition";
import { cn } from "$lib/cn";
import { getTrayDisplayMode, setTrayDisplayMode } from "$lib/tauriCommands";
import PowerPresetSettings from "./PowerPresetSettings.svelte";

interface Props {
	onclose: () => void;
	fanControlAvailable: boolean;
}

const { onclose, fanControlAvailable }: Props = $props();

let dialogEl: HTMLDivElement | undefined = $state(undefined);
let closeButtonEl: HTMLButtonElement | undefined = $state(undefined);
let trayDisplayMode: number = $state(0);

$effect(() => {
	closeButtonEl?.focus();
});

$effect(() => {
	getTrayDisplayMode()
		.then((mode) => {
			trayDisplayMode = mode;
		})
		.catch(() => {
			trayDisplayMode = 0;
		});
});

async function handleTrayModeChange(mode: number): Promise<void> {
	trayDisplayMode = mode;
	try {
		await setTrayDisplayMode(mode);
	} catch (error) {
		console.error("[PreferencesDialog] Failed to set tray display mode:", error);
	}
}

function handleKeydown(event: KeyboardEvent): void {
	if (event.key === "Escape") {
		onclose();
		return;
	}
	if (event.key === "Tab" && dialogEl) {
		const focusableSelector =
			'button:not([disabled]), select:not([disabled]), a:not([disabled]), [tabindex]:not([tabindex="-1"])';
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

const buttonBase =
	"cursor-pointer rounded-(--radius-button) border px-4 py-1.5 text-[12px] font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring) focus-visible:ring-offset-1 focus-visible:ring-offset-(--focus-ring-offset)";
const doneButton = cn(
	buttonBase,
	"border-(--border-subtle) bg-(--surface-elevated) text-(--text-primary) shadow-(--shadow-hairline) hover:bg-(--surface-2)",
);
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class={cn('fixed inset-0 z-50 flex items-center justify-center')}
  transition:fade={{ duration: 150 }}
>
  <button
    type="button"
    class={cn('absolute inset-0 bg-black/20 backdrop-blur-[1px] cursor-default')}
    onclick={onclose}
    aria-label="Close dialog"
    tabindex="-1"
  ></button>

  <div
    bind:this={dialogEl}
    class={cn(
      'relative w-[380px] rounded-(--radius-dialog) border border-(--border-subtle) bg-(--surface-elevated) shadow-(--shadow-elevated)'
    )}
    role="dialog"
    aria-modal="true"
    aria-label="Preferences"
    transition:scale={{ duration: 150, start: 0.95, opacity: 0 }}
  >
    <div class={cn('px-6 pt-5 pb-2')}>
      <h2 class={cn('text-[14px] font-semibold text-(--text-primary)')}>
        Preferences
      </h2>
    </div>

    <div class={cn('flex flex-col gap-5 px-6 py-3')}>
      <div class={cn('flex flex-col gap-2')}>
        <p class={cn('text-[12px] font-medium text-(--text-primary)')}>Menu bar display</p>
        <div class={cn('flex gap-0 rounded-(--radius-segmented) border border-(--border-subtle) bg-(--surface-2) p-0.5')} role="radiogroup" aria-label="Menu bar display">
          <button
            type="button"
            role="radio"
            aria-checked={trayDisplayMode === 0}
            class={cn(
              'flex-1 rounded-[5px] px-3 py-1 text-[12px] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring)',
              trayDisplayMode === 0
                ? 'bg-(--control-active-bg) text-(--control-active-text) font-medium'
                : 'text-(--text-secondary) hover:text-(--text-primary)'
            )}
            onclick={() => handleTrayModeChange(0)}
          >
            CPU Temperature
          </button>
          <button
            type="button"
            role="radio"
            aria-checked={trayDisplayMode === 1}
            class={cn(
              'flex-1 rounded-[5px] px-3 py-1 text-[12px] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring)',
              trayDisplayMode === 1
                ? 'bg-(--control-active-bg) text-(--control-active-text) font-medium'
                : 'text-(--text-secondary) hover:text-(--text-primary)'
            )}
            onclick={() => handleTrayModeChange(1)}
          >
            Fan RPM
          </button>
        </div>
      </div>

      <div class={cn('border-t border-(--border-subtle)')}></div>

      <div>
        {#if fanControlAvailable}
          <PowerPresetSettings />
        {:else}
          <p class={cn('text-[12px] text-(--text-secondary)')}>
            Fan preset automation is unavailable in TestFlight builds.
          </p>
        {/if}
      </div>
    </div>

    <div class={cn('flex justify-end px-6 pb-5 pt-2')}>
      <button
        type="button"
        bind:this={closeButtonEl}
        class={doneButton}
        onclick={onclose}
      >
        Done
      </button>
    </div>
  </div>
</div>
