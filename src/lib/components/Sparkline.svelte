<script lang="ts">
	interface Props {
		samples: (number | null)[];
		/** Shared y-axis max so rows are comparable. */
		max: number;
		capacity?: number;
		width?: number;
		height?: number;
	}
	let { samples, max, capacity = 60, width = 180, height = 22 }: Props = $props();

	const recent = $derived(samples.slice(-capacity));
	const step = $derived(width / capacity);
	const offset = $derived((capacity - recent.length) * step);

	// One polyline per run of received samples; gaps where probes were lost.
	const segments = $derived.by(() => {
		const out: string[] = [];
		let cur: string[] = [];
		recent.forEach((v, i) => {
			if (v == null) {
				if (cur.length) out.push(cur.join(" "));
				cur = [];
				return;
			}
			const x = offset + i * step + step / 2;
			const y = height - 2 - (Math.min(v, max) / (max || 1)) * (height - 4);
			cur.push(`${x.toFixed(1)},${y.toFixed(1)}`);
		});
		if (cur.length) out.push(cur.join(" "));
		return out;
	});

	const losses = $derived(
		recent.flatMap((v, i) => (v == null ? [offset + i * step + step / 2] : []))
	);
</script>

<svg {width} {height} class="block" aria-hidden="true">
	{#each losses as x (x)}
		<line x1={x} x2={x} y1={0} y2={height} class="stroke-red-500/70" stroke-width={Math.max(1, step - 1)} />
	{/each}
	{#each segments as points, i (i)}
		{#if points.includes(" ")}
			<polyline {points} fill="none" class="stroke-sky-500" stroke-width="1.25" stroke-linejoin="round" />
		{:else}
			{@const [cx, cy] = points.split(",")}
			<circle {cx} {cy} r="1.2" class="fill-sky-500" />
		{/if}
	{/each}
</svg>
