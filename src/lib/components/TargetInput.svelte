<script lang="ts">
	import { Input } from "$lib/components/ui/input";
	import { prefs } from "$lib/prefs.svelte";
	import { form } from "$lib/form.svelte";
	import XIcon from "@lucide/svelte/icons/x";
	import HistoryIcon from "@lucide/svelte/icons/history";

	let open = $state(false);
	let highlight = $state(-1);
	let el = $state<HTMLInputElement | null>(null);

	$effect(() => {
		form.input = el;
	});

	const matches = $derived(
		prefs.recentTargets.filter((t) => t !== form.target && t.toLowerCase().includes(form.target.trim().toLowerCase())).slice(0, 8)
	);

	function pick(t: string) {
		form.target = t;
		open = false;
		highlight = -1;
		el?.focus();
	}

	function onkeydown(e: KeyboardEvent) {
		if (!open || !matches.length) {
			if (e.key === "ArrowDown") open = true;
			return;
		}
		if (e.key === "ArrowDown") {
			e.preventDefault();
			highlight = (highlight + 1) % matches.length;
		} else if (e.key === "ArrowUp") {
			e.preventDefault();
			highlight = (highlight - 1 + matches.length) % matches.length;
		} else if (e.key === "Enter" && highlight >= 0) {
			e.preventDefault();
			pick(matches[highlight]);
		} else if (e.key === "Escape") {
			open = false;
		}
	}
</script>

<div class="relative max-w-sm flex-1">
	<Input
		bind:ref={el}
		bind:value={form.target}
		placeholder="Host or IP, e.g. 1.1.1.1 or example.com"
		class="h-8 font-mono text-[13px]"
		autocapitalize="off"
		autocorrect="off"
		spellcheck={false}
		autocomplete="off"
		role="combobox"
		aria-expanded={open && matches.length > 0}
		aria-controls="recent-targets"
		onfocus={() => (open = true)}
		onblur={() => setTimeout(() => (open = false), 120)}
		oninput={() => {
			open = true;
			highlight = -1;
		}}
		{onkeydown}
	/>
	{#if open && matches.length}
		<ul
			id="recent-targets"
			role="listbox"
			class="absolute top-full right-0 left-0 z-50 mt-1 overflow-hidden rounded-md border bg-popover p-1 text-sm text-popover-foreground shadow-md"
		>
			<li class="px-2 pt-0.5 pb-1 text-[11px] text-muted-foreground uppercase">Recent</li>
			{#each matches as t, i (t)}
				<li
					role="option"
					aria-selected={i === highlight}
					class="group flex items-center gap-2 rounded px-2 py-1 font-mono text-[13px] {i === highlight ? 'bg-accent text-accent-foreground' : 'hover:bg-accent/60'}"
					onmousedown={(e) => {
						e.preventDefault();
						pick(t);
					}}
				>
					<HistoryIcon class="size-3.5 text-muted-foreground" />
					<span class="flex-1 truncate">{t}</span>
					<button
						class="opacity-0 group-hover:opacity-60 hover:opacity-100"
						aria-label="Remove from recent"
						onmousedown={(e) => {
							e.preventDefault();
							e.stopPropagation();
							prefs.removeRecent(t);
						}}><XIcon class="size-3" /></button
					>
				</li>
			{/each}
		</ul>
	{/if}
</div>
