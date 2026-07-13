<script lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CheckCircle2, MoreHorizontal, Search } from "lucide-svelte";
import { cn } from "$lib/cn";
import type { SensorData as DesignTokenSensor } from "$lib/designTokens";
import { isAppStoreDistribution } from "$lib/distribution";
import {
	getAllSensorsForDisplay,
	isPerCoreTemperatureUnavailable,
	partitionDisplaySensors,
} from "$lib/sensorListPaneState";
import {
	getPrivilegeStatus,
	getTrayDisplayMode,
	hideToMenuBar,
	installHelper,
	listenCheckForUpdates,
	listenShowAbout,
	reconnectWriter,
	setTrayDisplayMode,
} from "$lib/tauriCommands";
import type { Sensor, SensorData } from "$lib/types";
import {
	buttonGhost,
	buttonSecondary,
	popoverSurface,
	telemetryGrid,
} from "$lib/ui";
import AboutDialog from "./AboutDialog.svelte";
import DisclosureGroup from "./DisclosureGroup.svelte";
import FanControlPane from "./FanControlPane.svelte";
import FanTelemetryRows from "./FanTelemetryRows.svelte";
import PreferencesDialog from "./PreferencesDialog.svelte";
import PresetDropdown from "./PresetDropdown.svelte";
import SensorListPane from "./SensorListPane.svelte";
import UpdateDialog from "./UpdateDialog.svelte";

interface Props {
	fans: DesignTokenSensor[];
	sensorData: SensorData | null;
}

const { fans, sensorData }: Props = $props();

let hasWriteAccess: boolean = $state(false);
let fanControlAvailable: boolean = $state(true);
let bannerMessage: string = $state("Fan control requires elevated privileges.");
let searchTerm: string = $state("");
let showMenu: boolean = $state(false);
let showPreferences: boolean = $state(false);
let showAbout: boolean = $state(false);
let showUpdate: boolean = $state(false);
let alwaysOnTop: boolean = $state(false);
let trayDisplayMode: number = $state(0);

const rawFans = $derived(sensorData?.fans ?? []);
const maxTemperatureC = $derived.by(() => {
	const temperatures = (sensorData?.details ?? [])
		.filter((sensor) => sensor.unit === "C")
		.flatMap((sensor) => (sensor.value === null ? [] : [sensor.value]));
	return temperatures.length === 0 ? null : Math.max(...temperatures);
});
const allDisplaySensors = $derived(
	sensorData ? getAllSensorsForDisplay(sensorData) : [],
);
const partitions = $derived(partitionDisplaySensors(allDisplaySensors));
const temperatureSensors = $derived(partitions.temperature);
const diskSensors = $derived(partitions.disk);
const perCoreUnavailable = $derived(
	isPerCoreTemperatureUnavailable(sensorData),
);
const modelLabel = $derived(sensorData?.diagnostics?.model_id ?? "Unknown Mac");
const fanAccessLabel = $derived(
	!fanControlAvailable
		? "Monitoring only"
		: hasWriteAccess
			? "Fan control enabled"
			: "Fan control needs access",
);
const searchQuery = $derived(searchTerm.trim().toLowerCase());
const filteredFans = $derived(
	searchQuery === ""
		? fans
		: fans.filter((fan) => fan.label.toLowerCase().includes(searchQuery)),
);
const filteredTemperatureSensors = $derived(
	filterSensors(temperatureSensors, searchQuery),
);
const filteredDiskSensors = $derived(filterSensors(diskSensors, searchQuery));

$effect(() => {
	getPrivilegeStatus()
		.then((status) => {
			hasWriteAccess = status.has_write_access;
			fanControlAvailable = status.fan_control_available;
			bannerMessage =
				status.reason ?? "Fan control requires elevated privileges.";
		})
		.catch(() => {
			hasWriteAccess = false;
			fanControlAvailable = false;
		});
});

const isDevMode = $derived(
	bannerMessage.includes("development mode") ||
		bannerMessage.includes("sudo pnpm"),
);

function filterSensors(sensors: Sensor[], query: string): Sensor[] {
	if (query === "") {
		return sensors;
	}

	return sensors.filter((sensor) =>
		[sensor.name, sensor.key, sensor.sensor_type].some((value) =>
			value.toLowerCase().includes(query),
		),
	);
}

async function handleGrantAccess(): Promise<void> {
	try {
		await installHelper();
		await reconnectWriter();
		hasWriteAccess = true;
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		if (!message.includes("cancelled") && !message.includes("canceled")) {
			bannerMessage = message;
		}
	}
}

async function handleAlwaysOnTop(): Promise<void> {
	try {
		const next = !alwaysOnTop;
		await getCurrentWindow().setAlwaysOnTop(next);
		alwaysOnTop = next;
		showMenu = false;
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		console.error("[DesktopDashboard] Failed to set always-on-top:", message);
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
		showMenu = false;
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		console.error("[DesktopDashboard] Failed to hide window:", message);
	}
}

async function handlePreferences(): Promise<void> {
	try {
		trayDisplayMode = await getTrayDisplayMode();
	} catch {
		trayDisplayMode = 0;
	}
	showMenu = false;
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

function openUpdateDialog(): void {
	showMenu = false;
	showUpdate = true;
}

function openAboutDialog(): void {
	showMenu = false;
	showAbout = true;
}
</script>

<section
  class={cn("h-full w-full overflow-hidden bg-(--window-background) text-(--text-primary)")}
>
  <div
    class={cn("flex h-full w-full min-w-0 flex-col overflow-hidden bg-(--window-background)")}
  >
    {#if !hasWriteAccess}
      <div
        class={cn(
          "flex shrink-0 items-center justify-between gap-2 border-b border-(--border-subtle) bg-(--material-toolbar) px-5 py-1.5 text-[11px] text-(--text-secondary)"
        )}
      >
        <span>{bannerMessage}</span>
        {#if fanControlAvailable && !isDevMode}
          <button
            type="button"
            class={cn(buttonSecondary, "shrink-0 px-2.5 py-0.5 text-[11px]")}
            onclick={handleGrantAccess}
          >
            Grant Access
          </button>
        {/if}
      </div>
    {/if}

    <header
      class={cn("grid min-h-[58px] shrink-0 grid-cols-[minmax(120px,1fr)_auto_minmax(180px,274px)_40px] items-center gap-3 border-b border-(--border-subtle) bg-(--material-toolbar) px-4")}
    >
      <h1 class={cn("text-[16px] font-semibold text-(--text-primary)")}>
        FanGuard
      </h1>
      <PresetDropdown />
      <label
        class={cn("flex h-8 min-w-0 items-center gap-2 rounded-[7px] border border-(--border-subtle) bg-(--material-control) px-2.5 text-(--text-secondary)")}
      >
        <Search size={17} strokeWidth={1.8} />
        <span class={cn("sr-only")}>Search sensors</span>
        <input
          type="search"
          bind:value={searchTerm}
          placeholder="Search"
          class={cn("min-w-0 grow bg-transparent text-[13px] text-(--text-primary) outline-none placeholder:text-(--text-muted)")}
        />
      </label>
      <div class={cn("relative")}>
        <button
          type="button"
          class={cn(buttonGhost, "h-8 w-8 px-0 rounded-[7px]")}
          aria-label="More actions"
          aria-expanded={showMenu}
          onclick={() => { showMenu = !showMenu; }}
        >
          <MoreHorizontal size={19} strokeWidth={1.8} />
        </button>
        {#if showMenu}
          <div
            class={cn(popoverSurface, "absolute right-0 top-10 z-20 w-48 p-1")}
            role="menu"
          >
            <button type="button" class={cn("w-full rounded-[8px] px-2.5 py-1.5 text-left text-[12px] text-(--text-primary) hover:bg-(--surface-hover)")} role="menuitem" onclick={handleAlwaysOnTop}>
              {alwaysOnTop ? "Unpin Window" : "Pin Window on Top"}
            </button>
            <button type="button" class={cn("w-full rounded-[8px] px-2.5 py-1.5 text-left text-[12px] text-(--text-primary) hover:bg-(--surface-hover)")} role="menuitem" onclick={handleHideToMenuBar}>
              Hide to Menu Bar
            </button>
            <button type="button" class={cn("w-full rounded-[8px] px-2.5 py-1.5 text-left text-[12px] text-(--text-primary) hover:bg-(--surface-hover)")} role="menuitem" onclick={handlePreferences}>
              Preferences...
            </button>
            {#if !isAppStoreDistribution()}
              <button type="button" class={cn("w-full rounded-[8px] px-2.5 py-1.5 text-left text-[12px] text-(--text-primary) hover:bg-(--surface-hover)")} role="menuitem" onclick={openUpdateDialog}>
                Check for Updates
              </button>
            {/if}
            <button type="button" class={cn("w-full rounded-[8px] px-2.5 py-1.5 text-left text-[12px] text-(--text-primary) hover:bg-(--surface-hover)")} role="menuitem" onclick={openAboutDialog}>
              About FanGuard
            </button>
          </div>
        {/if}
      </div>
    </header>

    <main class={cn("flex min-h-0 grow flex-col overflow-hidden bg-(--material-window)")}>
      <FanControlPane {rawFans} hasWriteAccess={hasWriteAccess} {maxTemperatureC} />

      <div class={cn("mx-4 mb-3 flex min-h-0 grow flex-col overflow-hidden rounded-[7px] border border-(--border-subtle) bg-(--surface-1)")}>
        <div class={cn("grid min-h-8 shrink-0 items-center border-b border-(--border-subtle) bg-(--surface-1) px-3 text-[12px] font-medium text-(--text-primary)", telemetryGrid)}>
          <div class={cn("pl-7 pr-3")}>Name</div>
          <div class={cn("border-l border-(--table-column-separator) px-3")}>Value</div>
          <div class={cn("border-l border-(--table-column-separator) px-3")}>Range</div>
          <div class={cn("border-l border-(--table-column-separator) px-3")}>Mode</div>
          <div class={cn("border-l border-(--table-column-separator) px-3")}>Status</div>
        </div>

        <div class={cn("min-h-0 grow overflow-y-auto bg-(--surface-1)")}>
          <DisclosureGroup title="Fans">
            <FanTelemetryRows fans={filteredFans} />
          </DisclosureGroup>

          <DisclosureGroup title="Temperature Sensors">
            {#if perCoreUnavailable}
              <p class={cn("border-b border-(--border-subtle) px-4 py-1.5 text-[11px] text-(--text-muted)")}>
                Per-core temperature not exposed on this Mac
              </p>
            {/if}
            <SensorListPane sensors={filteredTemperatureSensors} emptyLabel="No temperature sensors match this search." />
          </DisclosureGroup>

          <DisclosureGroup title="Disk Drives">
            <SensorListPane sensors={filteredDiskSensors} emptyLabel="No disk drives match this search." />
          </DisclosureGroup>
        </div>
      </div>
    </main>

    <footer
      class={cn("flex min-h-9 shrink-0 items-center gap-8 border-t border-(--border-subtle) bg-(--material-toolbar) px-4 text-[12px] text-(--text-secondary)")}
    >
      <span class={cn("flex items-center gap-2")}>
        <span class={cn("inline-block size-2.5 rounded-full bg-(--status-normal)")}></span>
        SMC Connected
      </span>
      <span>{modelLabel}</span>
      <span class={cn("flex items-center gap-2")}>
        <CheckCircle2 size={14} class={cn(hasWriteAccess ? "text-(--status-normal)" : "text-(--status-warm)")} strokeWidth={1.8} />
        {fanAccessLabel}
      </span>
      <span class={cn("ml-auto")}>Last updated just now</span>
    </footer>

  {#if showPreferences}
    <PreferencesDialog
      trayDisplayMode={trayDisplayMode}
      {fanControlAvailable}
      onTrayModeChange={handleTrayModeChange}
      onclose={() => { showPreferences = false; }}
    />
  {/if}

  {#if showAbout}
    <AboutDialog onclose={() => { showAbout = false; }} />
  {/if}

  {#if showUpdate}
    <UpdateDialog onclose={() => { showUpdate = false; }} />
  {/if}
  </div>
</section>
