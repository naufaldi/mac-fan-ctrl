<script lang="ts">
import { fade, scale } from "svelte/transition";
import { cn } from "$lib/cn";
import { checkForUpdate, type UpdateCheckResult } from "$lib/updater";

interface Props {
	onclose: () => void;
}

const { onclose }: Props = $props();

let result: UpdateCheckResult | null = $state(null);
let isChecking: boolean = $state(true);
let isInstalling: boolean = $state(false);
let dialogEl: HTMLDivElement | undefined = $state(undefined);

$effect(() => {
	checkForUpdate().then((r) => {
		result = r;
		isChecking = false;
	});
});

async function handleInstall(): Promise<void> {
	if (result?.status !== "available") return;
	isInstalling = true;
	try {
		await result.download();
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		result = { status: "error", message };
		isInstalling = false;
	}
}

function handleKeydown(event: KeyboardEvent): void {
	if (event.key === "Escape" && !isInstalling) {
		onclose();
	}
}

const buttonBase =
	"rounded-[5px] border px-4 py-1.5 text-[12px] font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500";
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
    disabled={isInstalling}
  ></button>

  <div
    bind:this={dialogEl}
    class={cn(
      'relative w-[360px] rounded-lg border border-gray-300 dark:border-[#4a4a4a] bg-[#ececec] dark:bg-[#2d2d2d] shadow-2xl'
    )}
    role="dialog"
    aria-modal="true"
    aria-label="Software Update"
    transition:scale={{ duration: 150, start: 0.95, opacity: 0 }}
  >
    <div class={cn('flex flex-col items-center px-6 pt-6 pb-2')}>
      <div class={cn('mb-3 flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-b from-green-400 to-green-600 shadow-lg')}>
        <svg class="h-6 w-6 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="7 10 12 15 17 10" />
          <line x1="12" y1="15" x2="12" y2="3" />
        </svg>
      </div>

      <h2 class={cn('text-[14px] font-semibold text-(--text-primary)')}>
        Software Update
      </h2>
    </div>

    <div class={cn('px-6 py-3 text-center')}>
      {#if isChecking}
        <p class={cn('text-[12px] text-(--text-secondary) animate-pulse')}>
          Checking for updates...
        </p>
      {:else if result?.status === "up-to-date"}
        <p class={cn('text-[12px] text-(--text-secondary)')}>
          You're running the latest version.
        </p>
      {:else if result?.status === "available"}
        <p class={cn('text-[12px] text-(--text-primary)')}>
          Version {result.version} is available.
        </p>
        {#if result.body}
          <div class={cn('mt-2 max-h-[120px] overflow-y-auto rounded border border-gray-200 dark:border-[#4a4a4a] bg-white dark:bg-[#1e1e1e] p-2 text-left text-[11px] text-(--text-secondary)')}>
            {result.body}
          </div>
        {/if}
        {#if isInstalling}
          <p class={cn('mt-2 text-[11px] text-blue-500 animate-pulse')}>
            Downloading and installing...
          </p>
        {/if}
      {:else if result?.status === "error"}
        <p class={cn('text-[12px] text-red-500 dark:text-red-400')}>
          {result.message}
        </p>
      {/if}
    </div>

    <div class={cn('flex justify-center gap-2 px-6 pb-5 pt-2')}>
      {#if result?.status === "available" && !isInstalling}
        <button
          type="button"
          class={cn(
            buttonBase,
            'border-gray-300 dark:border-[#4a4a4a] bg-white dark:bg-[#3a3a3a] text-(--text-primary) shadow-[0_1px_2px_rgba(0,0,0,0.05)] hover:bg-gray-50 dark:hover:bg-[#444]'
          )}
          onclick={onclose}
        >
          Later
        </button>
        <button
          type="button"
          class={cn(
            buttonBase,
            'border-blue-500 bg-blue-500 text-white shadow-[0_1px_2px_rgba(0,0,0,0.1)] hover:bg-blue-600'
          )}
          onclick={handleInstall}
        >
          Install &amp; Restart
        </button>
      {:else}
        <button
          type="button"
          class={cn(
            buttonBase,
            'border-gray-300 dark:border-[#4a4a4a] bg-white dark:bg-[#3a3a3a] text-(--text-primary) shadow-[0_1px_2px_rgba(0,0,0,0.05)] hover:bg-gray-50 dark:hover:bg-[#444]'
          )}
          onclick={onclose}
          disabled={isInstalling}
        >
          OK
        </button>
      {/if}
    </div>
  </div>
</div>
