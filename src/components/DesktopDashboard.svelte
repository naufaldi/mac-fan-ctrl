<script lang="ts">
import { cn } from "$lib/cn";
import type { SensorData as DesignTokenSensor } from "$lib/designTokens";
import type { SensorData } from "$lib/types";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getPrivilegeStatus, getTrayDisplayMode, openUrl, requestPrivilegeRestart, setTrayDisplayMode } from "$lib/tauriCommands";
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

let showPreferences: boolean = $state(false);
let trayDisplayMode: number = $state(0);

async function handleHideToMenuBar(): Promise<void> {
	try {
		await getCurrentWindow().hide();
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		console.error("[DesktopDashboard] Failed to hide window:", msg);
	}
}

async function handlePreferences(): Promise<void> {
	try {
		trayDisplayMode = await getTrayDisplayMode();
	} catch {
		trayDisplayMode = 0;
	}
	showPreferences = true;
}

async function handleTrayModeChange(mode: number): Promise<void> {
	trayDisplayMode = mode;
	try {
		await setTrayDisplayMode(mode);
	} catch (error) {
		console.error("[DesktopDashboard] Failed to set tray display mode:", error);
	}
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
    <button class={cn(chromeButtonClass)} type="button" onclick={handlePreferences}>
      Preferences...
    </button>
    <button class={cn(chromeButtonClass, 'w-8 font-serif italic')} type="button" aria-label="Help" onclick={handleHelp}>
      ?
    </button>
  </footer>
</section>

<!-- Preferences modal -->
{#if showPreferences}
  <div class={cn('fixed inset-0 z-50 flex items-center justify-center')}>
    <button
      type="button"
      class={cn('absolute inset-0 bg-black/30 cursor-default')}
      onclick={() => { showPreferences = false; }}
      aria-label="Close preferences"
      tabindex="-1"
    ></button>
    <div
      class={cn(
        'relative w-[340px] rounded-lg border border-gray-300 dark:border-[#4a4a4a] bg-[#ececec] dark:bg-[#2d2d2d] shadow-2xl'
      )}
      role="dialog"
      aria-modal="true"
      aria-label="Preferences"
    >
      <div class={cn('px-6 pt-5 pb-4')}>
        <h2 class={cn('text-[13px] font-semibold text-(--text-primary) mb-4')}>Preferences</h2>

        <div class={cn('space-y-3')}>
          <div>
            <p class={cn('text-[12px] font-medium text-(--text-primary) mb-2')}>Menu bar display</p>
            <div class={cn('flex gap-0 rounded-[5px] border border-gray-300 dark:border-[#4a4a4a] overflow-hidden')}>
              <button
                type="button"
                class={cn(
                  'flex-1 px-3 py-1.5 text-[12px] transition-colors',
                  trayDisplayMode === 0
                    ? 'bg-blue-500 text-white font-medium'
                    : 'bg-white dark:bg-[#3a3a3a] text-(--text-primary) hover:bg-gray-50 dark:hover:bg-[#444]'
                )}
                onclick={() => handleTrayModeChange(0)}
              >
                CPU Temperature
              </button>
              <button
                type="button"
                class={cn(
                  'flex-1 px-3 py-1.5 text-[12px] border-l border-gray-300 dark:border-[#4a4a4a] transition-colors',
                  trayDisplayMode === 1
                    ? 'bg-blue-500 text-white font-medium'
                    : 'bg-white dark:bg-[#3a3a3a] text-(--text-primary) hover:bg-gray-50 dark:hover:bg-[#444]'
                )}
                onclick={() => handleTrayModeChange(1)}
              >
                Fan RPM
              </button>
            </div>
          </div>
        </div>
      </div>

      <div class={cn('flex justify-end px-6 pb-5')}>
        <button
          type="button"
          class="rounded-[5px] border border-gray-300 dark:border-[#4a4a4a] bg-white dark:bg-[#3a3a3a] px-4 py-1.5 text-[12px] text-(--text-primary) shadow-[0_1px_2px_rgba(0,0,0,0.05)] hover:bg-gray-50 dark:hover:bg-[#444]"
          onclick={() => { showPreferences = false; }}
        >
          Done
        </button>
      </div>
    </div>
  </div>
{/if}
