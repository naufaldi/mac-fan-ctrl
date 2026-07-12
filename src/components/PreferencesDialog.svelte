<script lang="ts">
import { fade, scale } from "svelte/transition";
import { cn } from "$lib/cn";
import {
	backdropSurface,
	buttonSecondary,
	segmentedActive,
	segmentedInactive,
	sheetSurface,
} from "$lib/ui";
import PowerPresetSettings from "./PowerPresetSettings.svelte";

interface Props {
	trayDisplayMode: number;
	fanControlAvailable: boolean;
	onTrayModeChange: (mode: number) => void;
	onclose: () => void;
}

const { trayDisplayMode, fanControlAvailable, onTrayModeChange, onclose }: Props = $props();

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
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class={cn("fixed inset-0 z-50 flex items-center justify-center")}
  transition:fade={{ duration: 150 }}
>
  <button
    type="button"
    class={cn(backdropSurface)}
    onclick={onclose}
    aria-label="Close dialog"
    tabindex="-1"
  ></button>

  <div
    bind:this={dialogEl}
    class={cn(sheetSurface, "relative w-[380px]")}
    role="dialog"
    aria-modal="true"
    aria-label="Preferences"
    transition:scale={{ duration: 150, start: 0.96, opacity: 0 }}
  >
    <div class={cn("px-6 pt-5 pb-2")}>
      <h2 class={cn("text-[15px] font-semibold text-(--text-primary)")}>
        Preferences
      </h2>
    </div>

    <div class={cn("space-y-4 px-6 py-3")}>
      <div class={cn("flex flex-col gap-2")}>
        <p class={cn("text-[12px] font-medium text-(--text-primary)")}>Menu bar display</p>
        <div class={cn("flex gap-1.5")}>
          <button
            type="button"
            class={cn(trayDisplayMode === 0 ? segmentedActive : segmentedInactive, "px-3 py-1 text-[12px]")}
            aria-pressed={trayDisplayMode === 0}
            onclick={() => onTrayModeChange(0)}
          >
            CPU Temperature
          </button>
          <button
            type="button"
            class={cn(trayDisplayMode === 1 ? segmentedActive : segmentedInactive, "px-3 py-1 text-[12px]")}
            aria-pressed={trayDisplayMode === 1}
            onclick={() => onTrayModeChange(1)}
          >
            Fan RPM
          </button>
        </div>
      </div>

      <div class={cn("border-t border-(--border-subtle)")}></div>

      {#if fanControlAvailable}
        <PowerPresetSettings />
      {:else}
        <p class={cn("text-[12px] text-(--text-secondary)")}>Fan preset automation is unavailable in TestFlight builds.</p>
      {/if}
    </div>

    <div class={cn("flex justify-end px-6 pb-5 pt-2")}>
      <button
        type="button"
        bind:this={closeButtonEl}
        class={cn(buttonSecondary)}
        onclick={onclose}
      >
        Done
      </button>
    </div>
  </div>
</div>
