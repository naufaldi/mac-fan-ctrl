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
	"rounded-(--radius-button) border px-4 py-1.5 text-[12px] font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring) focus-visible:ring-offset-1 focus-visible:ring-offset-(--focus-ring-offset)";

const cancelButton = cn(
	buttonBase,
	"border-(--border-subtle) bg-(--surface-elevated) text-(--text-primary) shadow-(--shadow-hairline) hover:bg-(--surface-2)",
);

const primaryButton = cn(
	buttonBase,
	"border-(--control-active-border) bg-(--control-active-bg) text-(--control-active-text)",
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
    disabled={isInstalling}
  ></button>

  <div
    bind:this={dialogEl}
    class={cn(
      'relative w-[360px] rounded-(--radius-dialog) border border-(--border-subtle) bg-(--surface-elevated) shadow-(--shadow-elevated)'
    )}
    role="dialog"
    aria-modal="true"
    aria-label="Software Update"
    transition:scale={{ duration: 150, start: 0.95, opacity: 0 }}
  >
    <div class={cn('flex flex-col items-center px-6 pt-6 pb-2')}>
      <div class={cn('mb-3 flex h-12 w-12 items-center justify-center rounded-(--radius-card) border border-(--border-subtle) bg-(--surface-2)')}>
        <svg class="h-6 w-6 text-(--text-primary)" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
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
      {:else if result?.status === "managed-by-app-store"}
        <p class={cn('text-[12px] text-(--text-secondary)')}>
          Updates are managed by the App Store.
        </p>
      {:else if result?.status === "available"}
        <p class={cn('text-[12px] text-(--text-primary)')}>
          Version {result.version} is available.
        </p>
        {#if result.body}
          <div class={cn('mt-2 max-h-[120px] overflow-y-auto rounded-(--radius-input) border border-(--border-subtle) bg-(--surface-1) p-2 text-left text-[11px] text-(--text-secondary)')}>
            {result.body}
          </div>
        {/if}
        {#if isInstalling}
          <p class={cn('mt-2 text-[11px] text-(--text-secondary) animate-pulse')}>
            Downloading and installing...
          </p>
        {/if}
      {:else if result?.status === "error"}
        <p class={cn('text-[12px] text-(--text-secondary)')}>
          {result.message}
        </p>
      {/if}
    </div>

    <div class={cn('flex justify-center gap-2 px-6 pb-5 pt-2')}>
      {#if result?.status === "available" && !isInstalling}
        <button
          type="button"
          class={cancelButton}
          onclick={onclose}
        >
          Later
        </button>
        <button
          type="button"
          class={primaryButton}
          onclick={handleInstall}
        >
          Install &amp; Restart
        </button>
      {:else}
        <button
          type="button"
          class={cancelButton}
          onclick={onclose}
          disabled={isInstalling}
        >
          OK
        </button>
      {/if}
    </div>
  </div>
</div>
