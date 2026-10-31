<script lang="ts">
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import * as Select from "$lib/components/ui/select";
	import PlayIcon from "@lucide/svelte/icons/play";
	import { store } from "$lib/traces.svelte";
	import type { AddrFamily, Protocol } from "$lib/types";

	let target = $state("");
	let protocol = $state<Protocol>("icmp");
	let family = $state<AddrFamily>("any");
	let interval = $state("1000");
	let port = $state("");

	const protocols = [
		{ value: "icmp", label: "ICMP" },
		{ value: "udp", label: "UDP" },
		{ value: "tcp", label: "TCP" }
	];
	const families = [
		{ value: "any", label: "IPv4/6" },
		{ value: "v4", label: "IPv4" },
		{ value: "v6", label: "IPv6" }
	];
	const intervals = [
		{ value: "250", label: "0.25 s" },
		{ value: "500", label: "0.5 s" },
		{ value: "1000", label: "1 s" },
		{ value: "2000", label: "2 s" },
		{ value: "5000", label: "5 s" }
	];
	const labelOf = (list: { value: string; label: string }[], v: string) =>
		list.find((o) => o.value === v)?.label ?? v;

	function submit(e: SubmitEvent) {
		e.preventDefault();
		store.start({
			target: target.trim(),
			protocol,
			family,
			intervalMs: Number(interval),
			maxTtl: 30,
			packetSize: 84,
			maxRounds: null,
			port: protocol !== "icmp" && port ? Number(port) : null,
			resolveHostnames: true
		});
	}
</script>

<form class="flex items-center gap-2" onsubmit={submit}>
	<Input
		bind:value={target}
		placeholder="Host or IP — e.g. 1.1.1.1, example.com"
		class="h-8 max-w-sm flex-1 font-mono text-[13px]"
		autocapitalize="off"
		autocorrect="off"
		spellcheck={false}
	/>
	<Select.Root type="single" bind:value={protocol}>
		<Select.Trigger size="sm" class="w-20" aria-label="Protocol">{labelOf(protocols, protocol)}</Select.Trigger>
		<Select.Content>
			{#each protocols as o (o.value)}<Select.Item value={o.value} label={o.label} />{/each}
		</Select.Content>
	</Select.Root>
	<Select.Root type="single" bind:value={family}>
		<Select.Trigger size="sm" class="w-24" aria-label="Address family">{labelOf(families, family)}</Select.Trigger>
		<Select.Content>
			{#each families as o (o.value)}<Select.Item value={o.value} label={o.label} />{/each}
		</Select.Content>
	</Select.Root>
	<Select.Root type="single" bind:value={interval}>
		<Select.Trigger size="sm" class="w-20" aria-label="Interval">{labelOf(intervals, interval)}</Select.Trigger>
		<Select.Content>
			{#each intervals as o (o.value)}<Select.Item value={o.value} label={o.label} />{/each}
		</Select.Content>
	</Select.Root>
	{#if protocol !== "icmp"}
		<Input
			bind:value={port}
			placeholder={protocol === "tcp" ? "Port 80" : "Port"}
			inputmode="numeric"
			class="h-8 w-20 font-mono text-[13px]"
		/>
	{/if}
	<Button type="submit" size="sm" disabled={store.starting || !target.trim()}>
		<PlayIcon />Trace
	</Button>
</form>
