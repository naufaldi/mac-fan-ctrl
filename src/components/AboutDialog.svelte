<script lang="ts">
import { fade, scale } from "svelte/transition";
import { cn } from "$lib/cn";
import { type AppInfo, getAppInfo, openUrl } from "$lib/tauriCommands";

interface Props {
	onclose: () => void;
}

const { onclose }: Props = $props();

let appInfo: AppInfo | null = $state(null);
let dialogEl: HTMLDivElement | undefined = $state(undefined);
let closeButtonEl: HTMLButtonElement | undefined = $state(undefined);

$effect(() => {
	getAppInfo()
		.then((info) => {
			appInfo = info;
		})
		.catch(() => {
			appInfo = { name: "FanGuard", version: "unknown", identifier: "" };
		});
});

$effect(() => {
	closeButtonEl?.focus();
});

const GITHUB_REPO_URL = "https://github.com/naufaldi/mac-fan-ctrl";

async function handleGitHub(): Promise<void> {
	try {
		await openUrl(GITHUB_REPO_URL);
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		console.error("[AboutDialog] Failed to open URL:", msg);
	}
}

function handleKeydown(event: KeyboardEvent): void {
	if (event.key === "Escape") {
		onclose();
		return;
	}
	if (event.key === "Tab" && dialogEl) {
		const focusableSelector =
			'button:not([disabled]), a:not([disabled]), [tabindex]:not([tabindex="-1"])';
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
      'relative w-[320px] rounded-lg border border-gray-300 dark:border-[#4a4a4a] bg-[#ececec] dark:bg-[#2d2d2d] shadow-2xl'
    )}
    role="dialog"
    aria-modal="true"
    aria-label="About FanGuard"
    transition:scale={{ duration: 150, start: 0.95, opacity: 0 }}
  >
    <div class={cn('flex flex-col items-center px-6 pt-6 pb-2')}>
      <div class={cn('mb-3 flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-b from-blue-400 to-blue-600 shadow-lg')}>
        <svg class="h-8 w-8 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0" />
          <path d="M12 2v4" />
          <path d="M12 18v4" />
          <path d="M4.93 4.93l2.83 2.83" />
          <path d="M16.24 16.24l2.83 2.83" />
          <path d="M2 12h4" />
          <path d="M18 12h4" />
          <path d="M4.93 19.07l2.83-2.83" />
          <path d="M16.24 7.76l2.83-2.83" />
        </svg>
      </div>

      <h2 class={cn('text-[14px] font-semibold text-(--text-primary)')}>
        {appInfo?.name ?? "FanGuard"}
      </h2>

      <p class={cn('mt-1 text-[12px] font-mono text-(--text-secondary) [font-variant-numeric:tabular-nums]')}>
        Version {appInfo?.version ?? "..."}
      </p>
    </div>

    <div class={cn('flex flex-col items-center gap-1.5 px-6 py-3')}>
      <p class={cn('text-[11px] text-(--text-muted)')}>MIT License</p>
      <button
        type="button"
        class={cn('text-[11px] text-blue-500 hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300 hover:underline cursor-pointer')}
        onclick={handleGitHub}
      >
        View on GitHub
      </button>
    </div>

    <div class={cn('flex justify-center px-6 pb-5 pt-2')}>
      <button
        type="button"
        bind:this={closeButtonEl}
        class={cn(
          buttonBase,
          'border-gray-300 dark:border-[#4a4a4a] bg-white dark:bg-[#3a3a3a] text-(--text-primary) shadow-[0_1px_2px_rgba(0,0,0,0.05)] hover:bg-gray-50 dark:hover:bg-[#444]'
        )}
        onclick={onclose}
      >
        OK
      </button>
    </div>
  </div>
</div>
