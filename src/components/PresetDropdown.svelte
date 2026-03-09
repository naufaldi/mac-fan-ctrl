<script lang="ts">
import { cn } from "$lib/cn";
import {
	applyPreset,
	deletePreset,
	getActivePreset,
	getPresets,
	savePreset,
} from "$lib/tauriCommands";
import type { Preset } from "$lib/types";

// ── State ──────────────────────────────────────────────────────────────────

let presets: Preset[] = $state([]);
let activePresetName: string = $state("Automatic");
let isOpen: boolean = $state(false);
let isSaveDialogOpen: boolean = $state(false);
let savePresetName: string = $state("");
let saveError: string = $state("");
let applyError: string = $state("");
let deleteError: string = $state("");

// ── Load presets ───────────────────────────────────────────────────────────

async function refreshPresets(): Promise<void> {
	try {
		const [loadedPresets, activeName] = await Promise.all([
			getPresets(),
			getActivePreset(),
		]);
		presets = loadedPresets;
		activePresetName = activeName ?? "Automatic";
	} catch (error) {
		console.error("[mac-fan-ctrl] Failed to load presets:", error);
	}
}

$effect(() => {
	void refreshPresets();
});

// ── Handlers ───────────────────────────────────────────────────────────────

function toggleDropdown(): void {
	isOpen = !isOpen;
	if (isOpen) {
		void refreshPresets();
	}
}

function closeDropdown(): void {
	isOpen = false;
	applyError = "";
	deleteError = "";
}

async function handleApplyPreset(name: string): Promise<void> {
	applyError = "";
	deleteError = "";
	try {
		await applyPreset(name);
		activePresetName = name;
		closeDropdown();
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		console.error("[mac-fan-ctrl] Failed to apply preset:", error);
		applyError = `Failed to apply preset '${name}': ${msg}`;
	}
}

function openSaveDialog(): void {
	savePresetName = "";
	saveError = "";
	isSaveDialogOpen = true;
	closeDropdown();
}

async function handleSavePreset(): Promise<void> {
	const trimmed = savePresetName.trim();
	if (trimmed.length === 0) return;

	saveError = "";
	try {
		await savePreset(trimmed);
		activePresetName = trimmed;
		isSaveDialogOpen = false;
		void refreshPresets();
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		if (msg.startsWith("duplicate:")) {
			const existingName = msg.slice("duplicate:".length);
			saveError = `This configuration already exists as '${existingName}'`;
		} else {
			console.error("[mac-fan-ctrl] Failed to save preset:", error);
			saveError = "Failed to save preset";
		}
	}
}

async function handleDeletePreset(
	name: string,
	event: MouseEvent,
): Promise<void> {
	event.stopPropagation();
	applyError = "";
	deleteError = "";
	try {
		await deletePreset(name);
		void refreshPresets();
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		console.error("[mac-fan-ctrl] Failed to delete preset:", error);
		deleteError = `Failed to delete preset '${name}': ${msg}`;
	}
}

function handleKeydown(event: KeyboardEvent): void {
	if (event.key === "Escape") {
		closeDropdown();
		isSaveDialogOpen = false;
	}
}

// ── Derived ────────────────────────────────────────────────────────────────

const builtinPresets = $derived(presets.filter((p) => p.builtin));
const customPresets = $derived(presets.filter((p) => !p.builtin));

const chromeButtonClass =
	"rounded-[5px] border border-gray-300 dark:border-[#4a4a4a] bg-white dark:bg-[#3a3a3a] px-3 py-1 text-[12px] text-(--text-primary) shadow-[0_1px_2px_rgba(0,0,0,0.05)] transition-colors hover:bg-gray-50 dark:hover:bg-[#444] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500";

const dropdownItemClass =
	"w-full text-left px-3 py-1.5 text-[12px] text-(--text-primary) hover:bg-blue-500 hover:text-white transition-colors flex items-center justify-between";
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="relative">
  <button
    class={cn(chromeButtonClass)}
    type="button"
    onclick={toggleDropdown}
  >
    {activePresetName} ▾
  </button>

  {#if isOpen}
    <!-- Backdrop -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="fixed inset-0 z-40" onclick={closeDropdown}></div>

    <!-- Dropdown menu -->
    <div
      class={cn(
        'absolute top-full left-0 mt-1 z-50 min-w-[200px] rounded-md border border-gray-300 dark:border-[#4a4a4a] bg-white dark:bg-[#2d2d2d] shadow-lg overflow-hidden'
      )}
    >
      <!-- Built-in presets -->
      {#each builtinPresets as preset (preset.name)}
        <button
          type="button"
          class={cn(
            dropdownItemClass,
            preset.name === activePresetName ? 'font-semibold' : ''
          )}
          onclick={() => handleApplyPreset(preset.name)}
        >
          <span>{preset.name}</span>
          {#if preset.name === activePresetName}
            <span class="text-[10px]">✓</span>
          {/if}
        </button>
      {/each}

      <!-- Separator + custom presets -->
      {#if customPresets.length > 0}
        <div class="border-t border-gray-200 dark:border-[#4a4a4a]"></div>
        {#each customPresets as preset (preset.name)}
          <button
            type="button"
            class={cn(
              dropdownItemClass,
              preset.name === activePresetName ? 'font-semibold' : ''
            )}
            onclick={() => handleApplyPreset(preset.name)}
          >
            <span>{preset.name}</span>
            <span class="flex items-center gap-1">
              {#if preset.name === activePresetName}
                <span class="text-[10px]">✓</span>
              {/if}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <span
                role="button"
                tabindex="0"
                class="text-[10px] text-gray-400 hover:text-red-500 px-1 cursor-pointer"
                onclick={(e) => handleDeletePreset(preset.name, e)}
                aria-label={`Delete preset ${preset.name}`}
              >
                ✕
              </span>
            </span>
          </button>
        {/each}
      {/if}

      <!-- Apply/delete errors -->
      {#if applyError}
        <div class={cn("px-3 py-1.5")}>
          <p class={cn("text-[11px] text-red-500 dark:text-red-400")}>{applyError}</p>
        </div>
      {/if}
      {#if deleteError}
        <div class={cn("px-3 py-1.5")}>
          <p class={cn("text-[11px] text-red-500 dark:text-red-400")}>{deleteError}</p>
        </div>
      {/if}

      <!-- Save current -->
      <div class="border-t border-gray-200 dark:border-[#4a4a4a]"></div>
      <button
        type="button"
        class={cn(dropdownItemClass, 'text-(--text-muted)')}
        onclick={openSaveDialog}
      >
        Save current as...
      </button>
    </div>
  {/if}
</div>

<!-- Save dialog (mini-modal) -->
{#if isSaveDialogOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
    onclick={() => { isSaveDialogOpen = false; }}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class={cn(
        'w-[300px] rounded-lg border border-gray-300 dark:border-[#4a4a4a] bg-[#ececec] dark:bg-[#2d2d2d] shadow-2xl p-5'
      )}
      onclick={(e) => e.stopPropagation()}
    >
      <h3 class="text-[13px] font-semibold text-(--text-primary) mb-3">Save Preset</h3>
      <input
        type="text"
        bind:value={savePresetName}
        placeholder="Preset name"
        class={cn(
          'w-full rounded-[4px] border bg-white dark:bg-[#2a2a2a] px-2 py-1.5 text-[12px] text-(--text-primary) focus:outline-none focus:ring-2 focus:ring-blue-500',
          saveError ? 'border-red-400 dark:border-red-500 mb-1' : 'border-gray-300 dark:border-[#4a4a4a] mb-3'
        )}
        onkeydown={(e) => { if (e.key === 'Enter') { void handleSavePreset(); } }}
      />
      {#if saveError}
        <p class="text-[11px] text-red-500 dark:text-red-400 mb-2">{saveError}</p>
      {/if}
      <div class="flex justify-end gap-2">
        <button
          type="button"
          class={cn(chromeButtonClass)}
          onclick={() => { isSaveDialogOpen = false; }}
        >
          Cancel
        </button>
        <button
          type="button"
          class="rounded-[5px] border border-blue-500 bg-blue-500 px-3 py-1 text-[12px] font-medium text-white shadow-[0_1px_2px_rgba(0,0,0,0.1)] hover:bg-blue-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 disabled:opacity-50"
          disabled={savePresetName.trim().length === 0}
          onclick={handleSavePreset}
        >
          Save
        </button>
      </div>
    </div>
  </div>
{/if}
