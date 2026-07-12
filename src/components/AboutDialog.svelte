<script lang="ts">
import { Fan } from "lucide-svelte";
import { fade, scale } from "svelte/transition";
import { cn } from "$lib/cn";
import { type AppInfo, getAppInfo, openUrl } from "$lib/tauriCommands";
import { backdropSurface, buttonSecondary, sheetSurface } from "$lib/ui";

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
    class={cn(sheetSurface, "relative w-[320px]")}
    role="dialog"
    aria-modal="true"
    aria-label="About FanGuard"
    transition:scale={{ duration: 150, start: 0.96, opacity: 0 }}
  >
    <div class={cn("flex flex-col items-center px-6 pt-6 pb-2")}>
      <div class={cn("mb-3 flex h-14 w-14 items-center justify-center rounded-[16px] bg-(--surface-0)")}>
        <Fan size={26} class={cn("text-(--text-primary)")} strokeWidth={1.5} />
      </div>

      <h2 class={cn("text-[15px] font-semibold text-(--text-primary)")}>
        {appInfo?.name ?? "FanGuard"}
      </h2>

      <p class={cn("mt-1 font-mono text-[12px] tabular-nums text-(--text-secondary)")}>
        Version {appInfo?.version ?? "..."}
      </p>
    </div>

    <div class={cn("flex flex-col items-center gap-1.5 px-6 py-3")}>
      <p class={cn("text-[11px] text-(--text-muted)")}>MIT License</p>
      <button
        type="button"
        class={cn("text-[11px] text-(--text-secondary) underline-offset-2 hover:text-(--text-primary) hover:underline cursor-pointer")}
        onclick={handleGitHub}
      >
        View on GitHub
      </button>
    </div>

    <div class={cn("flex justify-center px-6 pb-5 pt-2")}>
      <button
        type="button"
        bind:this={closeButtonEl}
        class={cn(buttonSecondary)}
        onclick={onclose}
      >
        OK
      </button>
    </div>
  </div>
</div>
