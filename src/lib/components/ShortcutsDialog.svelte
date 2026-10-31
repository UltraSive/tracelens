<script lang="ts">
	import * as Dialog from "$lib/components/ui/dialog";
	import { Kbd } from "$lib/components/ui/kbd";
	import { modKey } from "$lib/format";

	let { open = $bindable(false) } = $props();

	const groups: [string, [string[], string][]][] = [
		["Traces", [
			[[modKey, "L"], "Focus the host box"],
			[[modKey, "T"], "New trace"],
			[[modKey, "W"], "Close tab"],
			[[modKey, "1–9"], "Go to tab"],
			[["Ctrl", "Tab"], "Next / previous tab (add ⇧)"],
		]],
		["Current trace", [
			[["Space"], "Pause / resume"],
			[["R"], "Reset statistics"],
			[[modKey, "S"], "Save to history"],
			[[modKey, "⇧", "C"], "Copy as Markdown"],
			[["G"], "Toggle latency chart"],
			[["P"], "Toggle path changes"],
			[["↑", "↓"], "Select hop"],
		]],
		["App", [
			[[modKey, "H"], "Saved traces"],
			[[modKey, "K"], "Compare traces"],
			[[modKey, "/"], "This list"],
		]]
	];
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header><Dialog.Title>Keyboard shortcuts</Dialog.Title></Dialog.Header>
		<div class="space-y-4 text-sm">
			{#each groups as [title, items] (title)}
				<div>
					<h3 class="mb-1 text-xs font-medium text-muted-foreground uppercase">{title}</h3>
					{#each items as [keys, label] (label)}
						<div class="flex items-center justify-between py-0.5">
							<span>{label}</span>
							<span class="flex gap-1">{#each keys as k (k)}<Kbd>{k}</Kbd>{/each}</span>
						</div>
					{/each}
				</div>
			{/each}
		</div>
	</Dialog.Content>
</Dialog.Root>
