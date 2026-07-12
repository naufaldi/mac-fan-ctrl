/**
 * Shared macOS Liquid Glass UI class tokens.
 * These helpers keep older component imports stable while the visual language
 * moves to compact native controls.
 */

export const buttonPrimary =
	"inline-flex min-h-7 items-center justify-center rounded-[8px] border border-(--accent-blue) bg-(--accent-blue) px-3 text-[12px] font-medium text-white transition-colors hover:bg-(--accent-blue-pressed) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring) focus-visible:ring-offset-1 focus-visible:ring-offset-(--focus-ring-offset) disabled:cursor-not-allowed disabled:opacity-45 cursor-pointer";

export const buttonSecondary =
	"inline-flex min-h-7 items-center justify-center rounded-[8px] border border-(--border-subtle) bg-(--material-control) px-3 text-[12px] font-medium text-(--text-primary) transition-colors hover:bg-(--surface-hover) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring) focus-visible:ring-offset-1 focus-visible:ring-offset-(--focus-ring-offset) disabled:cursor-not-allowed disabled:opacity-45 cursor-pointer";

export const buttonGhost =
	"inline-flex min-h-7 items-center justify-center rounded-[8px] px-2.5 text-[12px] font-medium text-(--text-secondary) transition-colors hover:bg-(--surface-hover) hover:text-(--text-primary) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring) focus-visible:ring-offset-1 focus-visible:ring-offset-(--focus-ring-offset) cursor-pointer";

export const segmentedActive =
	"inline-flex min-h-6 items-center justify-center rounded-[7px] bg-(--accent-blue) px-3 text-[12px] font-medium text-white shadow-[inset_0_0_0_0.5px_rgb(255_255_255_/_35%)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring) cursor-pointer";

export const segmentedInactive =
	"inline-flex min-h-6 items-center justify-center rounded-[7px] px-3 text-[12px] font-medium text-(--text-secondary) transition-colors hover:bg-(--surface-hover) hover:text-(--text-primary) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring) cursor-pointer";

export const sheetSurface =
	"rounded-[18px] border border-(--border-subtle) bg-(--material-sheet) shadow-[0_18px_48px_rgb(0_0_0_/_18%)]";

export const backdropSurface =
	"fixed inset-0 bg-black/20 backdrop-blur-[2px] cursor-default";

export const inputClass =
	"min-h-7 rounded-[8px] border border-(--border-subtle) bg-(--material-control) px-2.5 text-[12px] text-(--text-primary) [font-variant-numeric:tabular-nums] focus:outline-none focus:ring-2 focus:ring-(--focus-ring) focus:border-(--border-strong)";

export const popoverSurface =
	"rounded-[14px] border border-(--border-subtle) bg-(--material-popover) shadow-[0_12px_32px_rgb(0_0_0_/_16%)]";

export const telemetryGrid =
	"grid-cols-[minmax(220px,2fr)_minmax(120px,1fr)_minmax(130px,1fr)_minmax(100px,0.8fr)_minmax(100px,0.8fr)]";
