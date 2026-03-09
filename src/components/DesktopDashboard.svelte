<script lang="ts">
import { cn } from "$lib/cn";
import type { SensorData as DesignTokenSensor } from "$lib/designTokens";
import type { SensorData } from "$lib/types";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getPrivilegeStatus, openUrl, requestPrivilegeRestart } from "$lib/tauriCommands";
import FanControlPane from "./FanControlPane.svelte";
import PresetDropdown from "./PresetDropdown.svelte";
import SensorListPane from "./SensorListPane.svelte";

interface Props {
	fans: DesignTokenSensor[];
	sensorData: SensorData | null;
}

const { fans, sensorData }: Props = $props();

const rawFans = $derived(sensorData?.fans ?? []);
const sensors = $derived(sensorData?.details ?? []);

let hasWriteAccess: boolean = $state(true);
let bannerMessage: string = $state('Fan control requires elevated privileges.');

$effect(() => {
	getPrivilegeStatus()
		.then((status) => { hasWriteAccess = status.has_write_access; })
		.catch(() => { hasWriteAccess = false; });
});

const isDevMode = $derived(
	bannerMessage.includes('development mode') || bannerMessage.includes('sudo pnpm')
);

async function handleGrantAccess(): Promise<void> {
	try {
		await requestPrivilegeRestart();
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		if (!msg.includes('cancelled') && !msg.includes('canceled')) {
			bannerMessage = msg;
		}
	}
}

const GITHUB_REPO_URL = "https://github.com/nicholasgriffintn/mac-fan-ctrl";

let showComingSoon: boolean = $state(false);

async function handleHideToMenuBar(): Promise<void> {
	try {
		await getCurrentWindow().hide();
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		console.error("[DesktopDashboard] Failed to hide window:", msg);
	}
}

async function handlePreferences(): Promise<void> {
	showComingSoon = true;
	setTimeout(() => {
		showComingSoon = false;
	}, 2000);
}

async function handleHelp(): Promise<void> {
	try {
		await openUrl(GITHUB_REPO_URL);
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		console.error("[DesktopDashboard] Failed to open help URL:", msg);
	}
}

const chromeButtonClass =
	"rounded-[5px] border border-gray-300 dark:border-[#4a4a4a] bg-white dark:bg-[#3a3a3a] px-3 py-1 text-[12px] text-(--text-primary) shadow-[0_1px_2px_rgba(0,0,0,0.05)] transition-colors hover:bg-gray-50 dark:hover:bg-[#444] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500";
</script>

<section
  class={cn("flex h-full w-full flex-col overflow-hidden bg-[#ececec] dark:bg-[#1e1e1e] text-(--text-primary)")}
>
  {#if !hasWriteAccess}
    <div
      class={cn(
        "flex shrink-0 items-center justify-center gap-2 border-b border-amber-300 dark:border-amber-700 bg-amber-50 dark:bg-amber-900/30 px-4 py-1.5 text-[11px] text-amber-800 dark:text-amber-200"
      )}
    >
      <span>{bannerMessage}</span>
      {#if !isDevMode}
        <button
          type="button"
          class={cn(
            "rounded-[4px] border border-amber-400 dark:border-amber-600 bg-amber-100 dark:bg-amber-800/50 px-2 py-0.5 text-[11px] font-medium text-amber-900 dark:text-amber-100 hover:bg-amber-200 dark:hover:bg-amber-700/50 transition-colors"
          )}
          onclick={handleGrantAccess}
        >
          Grant Access
        </button>
      {/if}
    </div>
  {/if}

  <header
    class={cn(
      "flex shrink-0 items-center justify-center gap-2 border-b border-gray-300 dark:border-black/50 bg-[#ececec] dark:bg-[#2d2d2d] px-4 py-2"
    )}
  >
    <span class={cn("text-[12px] text-(--text-secondary)")}>Active preset:</span>
    <PresetDropdown />
  </header>

  <div class={cn("grid min-h-0 grow grid-cols-[1fr_300px] overflow-hidden bg-white dark:bg-[#1e1e1e]")}>
    <FanControlPane {fans} {rawFans} {sensors} {hasWriteAccess} />
    <SensorListPane {sensorData} />
  </div>

  <footer
    class={cn(
      "flex shrink-0 items-center justify-end gap-2 border-t border-gray-300 dark:border-black/50 bg-[#ececec] dark:bg-[#2d2d2d] px-4 py-2"
    )}
  >
    <button class={cn(chromeButtonClass)} type="button" onclick={handleHideToMenuBar}>
      Hide to menu bar
    </button>
    {#if showComingSoon}
      <span class={cn("text-[11px] text-(--text-secondary) animate-pulse")}>Coming soon</span>
    {/if}
    <button class={cn(chromeButtonClass)} type="button" onclick={handlePreferences}>
      Preferences...
    </button>
    <button class={cn(chromeButtonClass, 'w-8 font-serif italic')} type="button" aria-label="Help" onclick={handleHelp}>
      ?
    </button>
  </footer>
</section>
