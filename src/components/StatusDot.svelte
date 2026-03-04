<script lang="ts">
  import { cn } from '$lib/cn';
  import type { TemperatureStatus } from '$lib/designTokens';

  interface Props {
    status: TemperatureStatus;
    size?: 'sm' | 'md';
  }

  const { status, size = 'sm' }: Props = $props();

  const sizeClasses = {
    sm: 'w-2 h-2',
    md: 'w-3 h-3',
  };
  const statusColors: Record<TemperatureStatus, string> = {
    normal: 'var(--status-normal)',
    warm: 'var(--status-warm)',
    hot: 'var(--status-hot)',
    unknown: 'var(--status-unknown)',
  };
</script>

<span
  class={cn(sizeClasses[size], 'inline-block rounded-full ring-1 ring-(--border-strong)')}
  style="background-color: {statusColors[status]}"
  aria-hidden="true"
></span>
