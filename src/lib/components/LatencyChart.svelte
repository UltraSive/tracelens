<script lang="ts">
	import uPlot from "uplot";
	import "uplot/dist/uPlot.min.css";
	import type { HopStats } from "$lib/types";
	import { ownerName } from "$lib/format";

	interface Props {
		hops: HopStats[];
		intervalMs: number;
	}
	let { hops, intervalMs }: Props = $props();

	let container = $state<HTMLDivElement | null>(null);
	let width = $state(600);
	let height = $state(200);
	let plot: uPlot | null = null;
	let seriesKey = "";

	const palette = ["#0ea5e9", "#f97316", "#22c55e", "#a855f7", "#ef4444", "#eab308", "#14b8a6", "#ec4899", "#6366f1", "#84cc16"];

	function css(name: string) {
		return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || "#888";
	}

	const data = $derived.by((): uPlot.AlignedData => {
		const n = Math.max(0, ...hops.map((h) => h.samples.length));
		const step = intervalMs / 1000;
		const xs = Array.from({ length: n }, (_, i) => -(n - 1 - i) * step);
		const ys = hops.map((h) => {
			const pad = n - h.samples.length;
			return Array.from({ length: n }, (_, i) => (i < pad ? null : h.samples[i - pad]));
		});
		return [xs, ...ys] as uPlot.AlignedData;
	});

	function label(h: HopStats) {
		const host = h.hosts[0];
		const name = host ? (host.hostname ?? host.ip) : "???";
		const owner = ownerName(h);
		return `${h.ttl}. ${name}${owner ? ` (${owner})` : ""}`;
	}

	function build() {
		if (!container) return;
		plot?.destroy();
		const axis = { stroke: css("--muted-foreground"), grid: { stroke: css("--border"), width: 1 }, ticks: { stroke: css("--border") } };
		const opts: uPlot.Options = {
			width,
			height,
			cursor: { drag: { x: true, y: false } },
			scales: { x: { time: false } },
			legend: { show: hops.length <= 12 },
			axes: [
				{ ...axis, values: (_, v) => v.map((s) => (s === 0 ? "now" : `${Math.round(s)}s`)) },
				{ ...axis, label: "ms", size: 50 }
			],
			series: [
				{ label: "Time", value: (_, v) => (v == null ? "" : v === 0 ? "now" : `${v.toFixed(1)}s`) },
				...hops.map((h, i) => ({
					label: label(h),
					stroke: palette[i % palette.length],
					width: 1.5,
					spanGaps: false,
					points: { show: false },
					// idx is null when the cursor isn't over the plot
					value: (_: uPlot, v: number | null, _s: number, idx: number | null) =>
						idx == null ? "–" : v == null ? "lost" : `${v.toFixed(1)} ms`
				}))
			]
		};
		plot = new uPlot(opts, data, container);
		fit();
	}

	/** uPlot's height excludes the legend, so shrink the plot by the legend's height. */
	function fit() {
		if (!plot) return;
		const legend = plot.root.querySelector<HTMLElement>(".u-legend");
		plot.setSize({ width, height: Math.max(60, height - (legend?.offsetHeight ?? 0)) });
	}

	// Rebuild when the set of series changes; otherwise just push new data.
	$effect(() => {
		// labels include hostnames/owners that arrive after the first render
		const key = hops.map(label).join("\n") + `|${intervalMs}`;
		void width;
		void height;
		if (key !== seriesKey || !plot) {
			seriesKey = key;
			build();
		} else {
			fit();
		}
	});

	$effect(() => {
		plot?.setData(data);
	});

	$effect(() => () => plot?.destroy());
</script>

<div class="h-full w-full" bind:clientWidth={width} bind:clientHeight={height}>
	<div bind:this={container} class="latency-chart"></div>
</div>

<style>
	.latency-chart :global(.u-legend) {
		font-size: 11px;
		color: var(--muted-foreground);
	}
	.latency-chart :global(.u-select) {
		background: color-mix(in oklch, var(--primary) 12%, transparent);
	}
</style>
