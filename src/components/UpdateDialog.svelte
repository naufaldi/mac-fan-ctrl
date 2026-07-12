<script lang="ts">
import { Download } from "lucide-svelte";
import { fade, scale } from "svelte/transition";
import { cn } from "$lib/cn";
import { backdropSurface, buttonPrimary, buttonSecondary, sheetSurface } from "$lib/ui";
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
    disabled={isInstalling}
  ></button>

  <div
    bind:this={dialogEl}
    class={cn(sheetSurface, "relative w-[360px]")}
    role="dialog"
    aria-modal="true"
    aria-label="Software Update"
    transition:scale={{ duration: 150, start: 0.96, opacity: 0 }}
  >
    <div class={cn("flex flex-col items-center px-6 pt-6 pb-2")}>
      <div class={cn("mb-3 flex h-11 w-11 items-center justify-center rounded-[12px] bg-(--surface-0)")}>
        <Download size={20} class={cn("text-(--text-primary)")} strokeWidth={1.75} />
      </div>

      <h2 class={cn("text-[15px] font-semibold text-(--text-primary)")}>
        Software Update
      </h2>
    </div>

    <div class={cn("px-6 py-3 text-center")}>
      {#if isChecking}
        <p class={cn("animate-pulse text-[12px] text-(--text-secondary)")}>
          Checking for updates…
        </p>
      {:else if result?.status === "up-to-date"}
        <p class={cn("text-[12px] text-(--text-secondary)")}>
          You're running the latest version.
        </p>
      {:else if result?.status === "managed-by-app-store"}
        <p class={cn("text-[12px] text-(--text-secondary)")}>
          Updates are managed by the App Store.
        </p>
      {:else if result?.status === "available"}
        <p class={cn("text-[12px] text-(--text-primary)")}>
          Version {result.version} is available.
        </p>
        {#if result.body}
          <div class={cn("mt-2 max-h-[120px] overflow-y-auto rounded-[8px] border border-(--border-subtle) bg-(--surface-0) p-2 text-left text-[11px] text-(--text-secondary)")}>
            {result.body}
          </div>
        {/if}
        {#if isInstalling}
          <p class={cn("mt-2 animate-pulse text-[11px] text-(--text-secondary)")}>
            Downloading and installing…
          </p>
        {/if}
      {:else if result?.status === "error"}
        <p class={cn("text-[12px] text-(--text-primary)")}>
          {result.message}
        </p>
      {/if}
    </div>

    <div class={cn("flex justify-center gap-2 px-6 pb-5 pt-2")}>
      {#if result?.status === "available" && !isInstalling}
        <button
          type="button"
          class={cn(buttonSecondary)}
          onclick={onclose}
        >
          Later
        </button>
        <button
          type="button"
          class={cn(buttonPrimary)}
          onclick={handleInstall}
        >
          Install &amp; Restart
        </button>
      {:else}
        <button
          type="button"
          class={cn(buttonSecondary)}
          onclick={onclose}
          disabled={isInstalling}
        >
          OK
        </button>
      {/if}
    </div>
  </div>
</div>
