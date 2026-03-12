<script lang="ts">
import { fade, scale } from "svelte/transition";
import { cn } from "$lib/cn";
import PowerPresetSettings from "./PowerPresetSettings.svelte";

interface Props {
	onclose: () => void;
}

const { onclose }: Props = $props();

let dialogEl: HTMLDivElement | undefined = $state(undefined);
let closeButtonEl: HTMLButtonElement | undefined = $state(undefined);

$effect(() => {
	closeButtonEl?.focus();
});

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
	"cursor-pointer rounded-[5px] border px-4 py-1.5 text-[12px] font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500";
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class={cn('fixed inset-0 z-50 flex items-center justify-center')}
  transition:fade={{ duration: 150 }}
>
  <button
    type="button"
    class={cn('absolute inset-0 bg-black/30 backdrop-blur-[1px] cursor-default')}
    onclick={onclose}
    aria-label="Close dialog"
    tabindex="-1"
  ></button>

  <div
    bind:this={dialogEl}
    class={cn(
      'relative w-[380px] rounded-lg border border-gray-300 dark:border-[#4a4a4a] bg-[#ececec] dark:bg-[#2d2d2d] shadow-2xl'
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

    <div class={cn('px-6 py-3')}>
      <PowerPresetSettings />
    </div>

    <div class={cn('flex justify-end px-6 pb-5 pt-2')}>
      <button
        type="button"
        bind:this={closeButtonEl}
        class={cn(
          buttonBase,
          'border-gray-300 dark:border-[#4a4a4a] bg-white dark:bg-[#3a3a3a] text-(--text-primary) shadow-[0_1px_2px_rgba(0,0,0,0.05)] hover:bg-gray-50 dark:hover:bg-[#444]'
        )}
        onclick={onclose}
      >
        Done
      </button>
    </div>
  </div>
</div>
