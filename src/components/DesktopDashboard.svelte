<script lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { cn } from "$lib/cn";
import type { SensorData as DesignTokenSensor } from "$lib/designTokens";
import { isAppStoreDistribution } from "$lib/distribution";
import {
	getPrivilegeStatus,
	hideToMenuBar,
	installHelper,
	listenCheckForUpdates,
	listenShowAbout,
	reconnectWriter,
} from "$lib/tauriCommands";
import type { SensorData } from "$lib/types";
import AboutDialog from "./AboutDialog.svelte";
import FanControlPane from "./FanControlPane.svelte";
import PreferencesDialog from "./PreferencesDialog.svelte";
import PresetDropdown from "./PresetDropdown.svelte";
import SensorListPane from "./SensorListPane.svelte";
import UpdateDialog from "./UpdateDialog.svelte";

interface Props {
	fans: DesignTokenSensor[];
	sensorData: SensorData | null;
}

const { fans, sensorData }: Props = $props();

const rawFans = $derived(sensorData?.fans ?? []);
const sensors = $derived(sensorData?.details ?? []);
const fanControlRevision = $derived(
	sensorData?.fans
		.map((fan) => `${fan.index}:${fan.mode}:${Math.round(fan.target)}`)
		.join("|") ?? "",
);

let hasWriteAccess: boolean = $state(false);
let fanControlAvailable: boolean = $state(!isAppStoreDistribution());
let bannerMessage: string = $state("Fan control requires elevated privileges.");

$effect(() => {
	getPrivilegeStatus()
		.then((status) => {
			hasWriteAccess = status.has_write_access;
			fanControlAvailable = status.fan_control_available;
			if (status.reason) {
				bannerMessage = status.reason;
			}
		})
		.catch(() => {
			hasWriteAccess = false;
			fanControlAvailable = !isAppStoreDistribution();
		});
});

const isDevMode = $derived(
	bannerMessage.includes("development mode") ||
		bannerMessage.includes("sudo pnpm"),
);

async function handleGrantAccess(): Promise<void> {
	try {
		await installHelper();
		await reconnectWriter();
		hasWriteAccess = true;
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		if (!msg.includes("cancelled") && !msg.includes("canceled")) {
			bannerMessage = msg;
		}
	}
}

let showPreferences: boolean = $state(false);
let showAbout: boolean = $state(false);
let showUpdate: boolean = $state(false);
let alwaysOnTop: boolean = $state(false);

async function handleAlwaysOnTop(): Promise<void> {
	try {
		const next = !alwaysOnTop;
		await getCurrentWindow().setAlwaysOnTop(next);
		alwaysOnTop = next;
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		console.error("[DesktopDashboard] Failed to set always-on-top:", msg);
	}
}

$effect(() => {
	const unlistenPromise = listenShowAbout(() => {
		showAbout = true;
	});
	return () => {
		unlistenPromise.then((unlisten) => unlisten());
	};
});

$effect(() => {
	const unlistenPromise = listenCheckForUpdates(() => {
		showUpdate = true;
	});
	return () => {
		unlistenPromise.then((unlisten) => unlisten());
	};
});

async function handleHideToMenuBar(): Promise<void> {
	try {
		await hideToMenuBar();
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		console.error("[DesktopDashboard] Failed to hide window:", msg);
	}
}

function handlePreferences(): void {
	showPreferences = true;
}

const chromeButtonClass =
	"rounded-(--radius-button) border border-(--border-subtle) bg-(--surface-elevated) px-3 py-1 text-[12px] text-(--text-primary) shadow-(--shadow-hairline) transition-colors hover:bg-(--surface-2) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring) focus-visible:ring-offset-1 focus-visible:ring-offset-(--focus-ring-offset)";
const chromeButtonActiveClass =
	"border-(--control-active-border) bg-(--control-active-bg) text-(--control-active-text) font-medium shadow-none";
</script>

<section
  class={cn("flex h-full w-full flex-col overflow-hidden bg-(--surface-0) text-(--text-primary)")}
>
  {#if !hasWriteAccess && fanControlAvailable}
    <div
      class={cn(
        "flex shrink-0 items-center justify-center gap-2 border-b border-ember-orange/40 bg-(--surface-2) px-4 py-1.5 text-[11px] text-(--text-primary)"
      )}
    >
      <span>{bannerMessage}</span>
      {#if !isDevMode}
        <button
          type="button"
          class={cn(
            "rounded-(--radius-button) border border-(--border-subtle) bg-(--surface-elevated) px-2 py-0.5 text-[11px] font-medium text-(--text-primary) shadow-(--shadow-hairline) hover:bg-(--surface-2) transition-colors"
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
      "flex shrink-0 items-center justify-center gap-2 border-b border-(--border-subtle) bg-(--surface-0) px-4 py-2"
    )}
  >
    {#if fanControlAvailable}
      <span class={cn("text-[12px] text-(--text-secondary)")}>Active preset:</span>
      <PresetDropdown />
    {:else}
      <span class={cn("text-[12px] text-(--text-secondary)")}>Monitoring only</span>
    {/if}
  </header>

  <div class={cn("grid min-h-0 grow grid-cols-[1fr_300px] overflow-hidden bg-(--surface-1)")}>
    {#key fanControlRevision}
      <FanControlPane {fans} {rawFans} {sensors} {hasWriteAccess} {fanControlAvailable} />
    {/key}
    <SensorListPane {sensorData} />
  </div>

  <footer
    class={cn(
      "flex shrink-0 items-center gap-2 border-t border-(--border-subtle) bg-(--surface-0) px-4 py-2"
    )}
  >
    <button
      class={cn(
        chromeButtonClass,
        alwaysOnTop && chromeButtonActiveClass
      )}
      type="button"
      aria-label={alwaysOnTop ? "Unpin window" : "Pin window on top"}
      aria-pressed={alwaysOnTop}
      onclick={handleAlwaysOnTop}
    >
      {alwaysOnTop ? "Pinned" : "Pin on Top"}
    </button>
    <div class="grow"></div>
    <button class={cn(chromeButtonClass)} type="button" onclick={handleHideToMenuBar}>
      Hide to menu bar
    </button>
    <button class={cn(chromeButtonClass)} type="button" onclick={handlePreferences}>
      Preferences...
    </button>
    {#if !isAppStoreDistribution()}
      <button class={cn(chromeButtonClass)} type="button" onclick={() => { showUpdate = true; }}>
        Check for Updates
      </button>
    {/if}
    <button class={cn(chromeButtonClass)} type="button" aria-label="About" onclick={() => { showAbout = true; }}>
      About
    </button>
  </footer>

  {#if showPreferences}
    <PreferencesDialog {fanControlAvailable} onclose={() => { showPreferences = false; }} />
  {/if}

  {#if showAbout}
    <AboutDialog onclose={() => { showAbout = false; }} />
  {/if}

  {#if showUpdate}
    <UpdateDialog onclose={() => { showUpdate = false; }} />
  {/if}
</section>
