<script lang="ts">
import { Check, X } from "lucide-svelte";
import { cn } from "$lib/cn";
import {
	applyPreset,
	deletePreset,
	getActivePreset,
	getPresets,
	savePreset,
} from "$lib/tauriCommands";
import type { Preset } from "$lib/types";

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

const popupButtonClass =
	"rounded-(--radius-button) border border-(--border-subtle) bg-(--surface-elevated) px-3 py-1 text-[12px] text-(--text-primary) shadow-(--shadow-hairline) transition-colors hover:bg-(--surface-2) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring) focus-visible:ring-offset-1 focus-visible:ring-offset-(--focus-ring-offset)";

const dropdownItemClass =
	"w-full text-left px-3 py-1.5 text-[12px] text-(--text-primary) hover:bg-(--surface-2) transition-colors flex items-center justify-between";

const buttonBase =
	"cursor-pointer rounded-(--radius-button) border px-4 py-1.5 text-[12px] font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring) focus-visible:ring-offset-1 focus-visible:ring-offset-(--focus-ring-offset)";

const cancelButton = cn(
	buttonBase,
	"border-(--border-subtle) bg-(--surface-elevated) text-(--text-primary) shadow-(--shadow-hairline) hover:bg-(--surface-2)",
);

const primaryButton = cn(
	buttonBase,
	"border-(--control-active-border) bg-(--control-active-bg) text-(--control-active-text) disabled:opacity-50",
);
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="relative">
  <button
    class={cn(popupButtonClass)}
    type="button"
    onclick={toggleDropdown}
    aria-haspopup="menu"
    aria-expanded={isOpen}
    aria-label="Select fan preset"
  >
    {activePresetName} ▾
  </button>

  {#if isOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="fixed inset-0 z-40" onclick={closeDropdown}></div>

    <div
      role="menu"
      aria-label="Fan presets"
      class={cn(
        'absolute top-full left-0 mt-1 z-50 min-w-[200px] rounded-(--radius-dialog) border border-(--border-subtle) bg-(--surface-elevated) shadow-(--shadow-elevated) overflow-hidden'
      )}
    >
      {#each builtinPresets as preset, i (preset.name)}
        <button
          type="button"
          role="menuitem"
          class={cn(
            dropdownItemClass,
            preset.name === activePresetName ? 'font-medium' : '',
            focusedIndex === i ? 'bg-(--surface-2)' : ''
          )}
          onclick={() => handleApplyPreset(preset.name)}
        >
          <span>{preset.name}</span>
          {#if preset.name === activePresetName}
            <Check size={12} class="text-(--text-primary)" aria-hidden="true" />
          {/if}
        </button>
      {/each}

      {#if customPresets.length > 0}
        <div class="border-t border-(--border-subtle)"></div>
        {#each customPresets as preset, i (preset.name)}
          <button
            type="button"
            role="menuitem"
            class={cn(
              dropdownItemClass,
              preset.name === activePresetName ? 'font-medium' : '',
              focusedIndex === builtinPresets.length + i ? 'bg-(--surface-2)' : ''
            )}
            onclick={() => handleApplyPreset(preset.name)}
          >
            <span>{preset.name}</span>
            <span class="flex items-center gap-1">
              {#if preset.name === activePresetName}
                <Check size={12} class="text-(--text-primary)" aria-hidden="true" />
              {/if}
              <span
                role="button"
                tabindex="0"
                class="text-(--text-muted) hover:text-(--text-primary) px-1 cursor-pointer"
                onclick={(e) => handleDeletePreset(preset.name, e)}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleDeletePreset(preset.name, e); }}
                aria-label={`Delete preset ${preset.name}`}
              >
                <X size={12} aria-hidden="true" />
              </span>
            </span>
          </button>
        {/each}
      {/if}

      {#if applyError}
        <div class={cn("px-3 py-1.5 border-t border-(--border-subtle)")}>
          <p class={cn("text-[11px] text-(--text-secondary)")}>{applyError}</p>
        </div>
      {/if}
      {#if deleteError}
        <div class={cn("px-3 py-1.5 border-t border-(--border-subtle)")}>
          <p class={cn("text-[11px] text-(--text-secondary)")}>{deleteError}</p>
        </div>
      {/if}

      <div class="border-t border-(--border-subtle)"></div>
      <button
        type="button"
        role="menuitem"
        class={cn(
          dropdownItemClass,
          'text-(--text-muted)',
          focusedIndex === allMenuItems.length ? 'bg-(--surface-2)' : ''
        )}
        onclick={openSaveDialog}
      >
        Save current as...
      </button>
    </div>
  {/if}
</div>

{#if isSaveDialogOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/20"
    onclick={() => { isSaveDialogOpen = false; }}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class={cn(
        'w-[300px] rounded-(--radius-dialog) border border-(--border-subtle) bg-(--surface-elevated) shadow-(--shadow-elevated) p-5'
      )}
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-label="Save preset"
    >
      <h3 class="text-[13px] font-semibold text-(--text-primary) mb-3">Save Preset</h3>
      <input
        type="text"
        bind:value={savePresetName}
        placeholder="Preset name"
        aria-label="Preset name"
        class={cn(
          'w-full rounded-(--radius-input) border bg-(--surface-elevated) px-2 py-1.5 text-[12px] text-(--text-primary) focus:outline-none focus:ring-2 focus:ring-(--focus-ring)',
          saveError ? 'border-(--color-ember-orange) mb-1' : 'border-(--border-subtle) mb-3'
        )}
        onkeydown={(e) => { if (e.key === 'Enter') { void handleSavePreset(); } }}
      />
      {#if saveError}
        <p class="text-[11px] text-(--text-secondary) mb-2">{saveError}</p>
      {/if}
      <div class="flex justify-end gap-2">
        <button
          type="button"
          class={cancelButton}
          onclick={() => { isSaveDialogOpen = false; }}
        >
          Cancel
        </button>
        <button
          type="button"
          class={primaryButton}
          disabled={savePresetName.trim().length === 0}
          onclick={handleSavePreset}
        >
          Save
        </button>
      </div>
    </div>
  </div>
{/if}
