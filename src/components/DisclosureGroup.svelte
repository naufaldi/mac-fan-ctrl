<script lang="ts">
import { ChevronDown, Fan, HardDrive, Thermometer } from "lucide-svelte";
import type { Snippet } from "svelte";
import { cn } from "$lib/cn";

interface Props {
	title: string;
	count?: number;
	children: Snippet;
}

const { title, count, children }: Props = $props();

let open: boolean = $state(true);
const GroupIcon = $derived(getGroupIcon(title));

function getGroupIcon(title: string) {
	if (title.toLowerCase().includes("fan")) return Fan;
	if (title.toLowerCase().includes("disk")) return HardDrive;
	return Thermometer;
}
</script>

<section class={cn("border-b border-(--border-subtle) last:border-b-0")}>
  <button
    type="button"
    class={cn(
      "flex min-h-9 w-full items-center justify-between bg-(--table-section-background) px-3 text-left transition-colors hover:bg-(--surface-hover) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring) focus-visible:ring-inset cursor-pointer"
    )}
    aria-expanded={open}
    onclick={() => { open = !open; }}
  >
    <span class={cn("flex items-center gap-3")}>
      <ChevronDown
        size={16}
        class={cn("text-(--text-secondary) transition-transform duration-150", open ? "rotate-0" : "-rotate-90")}
        strokeWidth={1.75}
      />
      <GroupIcon size={18} class={cn("text-(--text-secondary)")} strokeWidth={1.75} />
      <span class={cn("text-[14px] font-medium uppercase text-(--text-secondary)")}>
        {title}
      </span>
    </span>
    {#if count !== undefined}
      <span class={cn("font-mono text-[11px] tabular-nums text-(--text-muted)")}>
        {count}
      </span>
    {/if}
  </button>

  {#if open}
    <div class={cn("flex flex-col")}>
      {@render children()}
    </div>
  {/if}
</section>
