<script lang="ts">
  import { cn } from "$lib/cn";

  interface Props {
    data: number[];
    statusColor: string;
    width?: number;
    height?: number;
  }

  const {
    data,
    statusColor,
    width = 60,
    height = 24,
  }: Props = $props();

  function generatePath(points: number[]): string {
    if (points.length < 2) return '';

    const min = Math.min(...points);
    const max = Math.max(...points);
    const range = max - min || 1;

    return points
      .map((point, i) => {
        const x = (i / (points.length - 1)) * width;
        const y = height - ((point - min) / range) * height;
        return `${i === 0 ? 'M' : 'L'} ${x} ${y}`;
      })
      .join(' ');
  }

  const path = $derived(generatePath(data));
</script>

<svg {width} {height} class={cn("overflow-visible")}>
  <path
    d={path}
    fill="none"
    stroke={statusColor}
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  />
</svg>
