<script lang="ts">
import { ChevronDown } from "lucide-svelte";
import { cn } from "$lib/cn";
import {
	applyPreset,
	deletePreset,
	getActivePreset,
	getPresets,
	savePreset,
} from "$lib/tauriCommands";
import type { Preset } from "$lib/types";
import { buttonPrimary, buttonSecondary, inputClass, sheetSurface } from "$lib/ui";

let presets: Preset[] = $state([]);
let activePresetName: string = $state("Automatic");
let isOpen: boolean = $state(false);
let isSaveDialogOpen: boolean = $state(false);
let savePresetName: string = $state("");
let saveError: string = $state("");
let applyError: string = $state("");
let deleteError: string = $state("");
let focusedIndex: number = $state(-1);

async function refreshPresets(): Promise<void> {
	try {
		const [loadedPresets, activeName] = await Promise.all([
			getPresets(),
			getActivePreset(),
		]);
		presets = loadedPresets;
		activePresetName = activeName ?? "Automatic";
	} catch (error) {
		console.error("[fanguard] Failed to load presets:", error);
	}
}

$effect(() => {
	void refreshPresets();
});

function toggleDropdown(): void {
	isOpen = !isOpen;
	if (isOpen) {
		focusedIndex = -1;
		void refreshPresets();
	}
}

function closeDropdown(): void {
	isOpen = false;
	focusedIndex = -1;
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
		console.error("[fanguard] Failed to apply preset:", error);
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
			console.error("[fanguard] Failed to save preset:", error);
			saveError = "Failed to save preset";
		}
	}
}

async function handleDeletePreset(
	name: string,
	event: Event,
): Promise<void> {
	event.stopPropagation();
	applyError = "";
	deleteError = "";
	try {
		await deletePreset(name);
		void refreshPresets();
	} catch (error) {
		const msg = error instanceof Error ? error.message : String(error);
		console.error("[fanguard] Failed to delete preset:", error);
		deleteError = `Failed to delete preset '${name}': ${msg}`;
	}
}

const builtinPresets = $derived(presets.filter((p) => p.builtin));
const customPresets = $derived(presets.filter((p) => !p.builtin));
const allMenuItems = $derived([...builtinPresets, ...customPresets]);

function handleKeydown(event: KeyboardEvent): void {
	if (event.key === "Escape") {
		closeDropdown();
		isSaveDialogOpen = false;
		return;
	}

	if (!isOpen) return;

	const itemCount = allMenuItems.length + 1;

	if (event.key === "ArrowDown") {
		event.preventDefault();
		focusedIndex = focusedIndex < itemCount - 1 ? focusedIndex + 1 : 0;
	} else if (event.key === "ArrowUp") {
		event.preventDefault();
		focusedIndex = focusedIndex > 0 ? focusedIndex - 1 : itemCount - 1;
	} else if (event.key === "Home") {
		event.preventDefault();
		focusedIndex = 0;
	} else if (event.key === "End") {
		event.preventDefault();
		focusedIndex = itemCount - 1;
	} else if (event.key === "Enter" && focusedIndex >= 0) {
		event.preventDefault();
		if (focusedIndex < allMenuItems.length) {
			void handleApplyPreset(allMenuItems[focusedIndex].name);
		} else {
			openSaveDialog();
		}
	}
}

const itemClass =
	"w-full text-left px-3 py-1.5 text-[12px] text-(--text-primary) hover:bg-(--surface-hover) transition-colors flex items-center justify-between cursor-pointer";
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="relative">
  <button
    class={cn(buttonSecondary, "h-10 gap-2 rounded-[10px] px-3.5 text-[16px]")}
    type="button"
    onclick={toggleDropdown}
    aria-haspopup="menu"
    aria-expanded={isOpen}
    aria-label="Select fan preset"
  >
    Preset: {activePresetName}
    <ChevronDown size={13} class={cn("text-(--text-secondary)")} strokeWidth={1.75} />
  </button>

  {#if isOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="fixed inset-0 z-40" onclick={closeDropdown}></div>

    <div
      role="menu"
      aria-label="Fan presets"
      class={cn(
        sheetSurface,
        "absolute top-full left-0 mt-1 z-50 min-w-[220px] overflow-hidden py-1"
      )}
    >
      {#each builtinPresets as preset, i (preset.name)}
        <button
          type="button"
          role="menuitem"
          class={cn(
            itemClass,
            preset.name === activePresetName && "font-medium",
            focusedIndex === i && "bg-(--surface-hover)"
          )}
          onclick={() => handleApplyPreset(preset.name)}
        >
          <span>{preset.name}</span>
          {#if preset.name === activePresetName}
            <span class={cn("text-[10px] text-(--text-secondary)")}>✓</span>
          {/if}
        </button>
      {/each}

      {#if customPresets.length > 0}
        <div class={cn("my-1 border-t border-(--border-subtle)")}></div>
        {#each customPresets as preset, i (preset.name)}
          <button
            type="button"
            role="menuitem"
            class={cn(
              itemClass,
              preset.name === activePresetName && "font-medium",
              focusedIndex === builtinPresets.length + i && "bg-(--surface-hover)"
            )}
            onclick={() => handleApplyPreset(preset.name)}
          >
            <span>{preset.name}</span>
            <span class={cn("flex items-center gap-1")}>
              {#if preset.name === activePresetName}
                <span class={cn("text-[10px] text-(--text-secondary)")}>✓</span>
              {/if}
              <span
                role="button"
                tabindex="0"
                class={cn("text-[10px] text-(--text-muted) px-1 hover:text-(--text-primary) cursor-pointer")}
                onclick={(e) => handleDeletePreset(preset.name, e)}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleDeletePreset(preset.name, e); }}
                aria-label={`Delete preset ${preset.name}`}
              >
                ✕
              </span>
            </span>
          </button>
        {/each}
      {/if}

      {#if applyError}
        <div class={cn("px-3 py-1.5")}>
          <p class={cn("text-[11px] text-(--status-hot)")}>{applyError}</p>
        </div>
      {/if}
      {#if deleteError}
        <div class={cn("px-3 py-1.5")}>
          <p class={cn("text-[11px] text-(--status-hot)")}>{deleteError}</p>
        </div>
      {/if}

      <div class={cn("my-1 border-t border-(--border-subtle)")}></div>
      <button
        type="button"
        role="menuitem"
        class={cn(
          itemClass,
          "text-(--text-secondary)",
          focusedIndex === allMenuItems.length && "bg-(--surface-hover)"
        )}
        onclick={openSaveDialog}
      >
        Save current as…
      </button>
    </div>
  {/if}
</div>

{#if isSaveDialogOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class={cn("fixed inset-0 z-50 flex items-center justify-center bg-black/25 backdrop-blur-[1px]")}
    onclick={() => { isSaveDialogOpen = false; }}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class={cn(sheetSurface, "w-[300px] p-5")}
      onclick={(e) => e.stopPropagation()}
    >
      <h3 class={cn("mb-3 text-[13px] font-medium text-(--text-primary)")}>Save Preset</h3>
      <input
        type="text"
        bind:value={savePresetName}
        placeholder="Preset name"
        aria-label="Preset name"
        class={cn(inputClass, "w-full", saveError ? "mb-1" : "mb-3")}
        onkeydown={(e) => { if (e.key === 'Enter') { void handleSavePreset(); } }}
      />
      {#if saveError}
        <p class={cn("mb-2 text-[11px] text-(--status-hot)")}>{saveError}</p>
      {/if}
      <div class={cn("flex justify-end gap-2")}>
        <button
          type="button"
          class={cn(buttonSecondary)}
          onclick={() => { isSaveDialogOpen = false; }}
        >
          Cancel
        </button>
        <button
          type="button"
          class={cn(buttonPrimary)}
          disabled={savePresetName.trim().length === 0}
          onclick={handleSavePreset}
        >
          Save
        </button>
      </div>
    </div>
  </div>
{/if}
