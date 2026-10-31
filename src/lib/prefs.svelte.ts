import { invoke } from "@tauri-apps/api/core";
import type { Preset } from "./types";

export type BottomPanel = "none" | "chart" | "paths";
export type ChartMode = "hop" | "all";

const MAX_RECENT = 15;

interface PrefsData {
	recentTargets: string[];
	presets: Preset[];
	columns: string[] | null;
	panel: BottomPanel;
	chartMode: ChartMode;
	advancedOpen: boolean;
}

class Prefs {
	recentTargets = $state<string[]>([]);
	presets = $state<Preset[]>([]);
	/** null = default column set */
	columns = $state<string[] | null>(null);
	panel = $state<BottomPanel>("none");
	chartMode = $state<ChartMode>("hop");
	advancedOpen = $state(false);

	#loaded = false;
	#timer: ReturnType<typeof setTimeout> | undefined;

	async load() {
		try {
			const p = await invoke<Partial<PrefsData>>("get_prefs");
			this.recentTargets = p.recentTargets ?? [];
			this.presets = p.presets ?? [];
			this.columns = p.columns ?? null;
			this.panel = p.panel ?? "none";
			this.chartMode = p.chartMode ?? "hop";
			this.advancedOpen = p.advancedOpen ?? false;
		} catch (e) {
			console.warn("could not load prefs", e);
		}
		this.#loaded = true;
	}

	/** Debounced write; call after any change. */
	save() {
		if (!this.#loaded) return;
		clearTimeout(this.#timer);
		this.#timer = setTimeout(() => {
			const data: PrefsData = {
				recentTargets: $state.snapshot(this.recentTargets),
				presets: $state.snapshot(this.presets),
				columns: $state.snapshot(this.columns),
				panel: this.panel,
				chartMode: this.chartMode,
				advancedOpen: this.advancedOpen
			};
			invoke("set_prefs", { prefs: data }).catch((e) => console.warn("could not save prefs", e));
		}, 300);
	}

	addRecent(target: string) {
		this.recentTargets = [target, ...this.recentTargets.filter((t) => t !== target)].slice(0, MAX_RECENT);
		this.save();
	}

	removeRecent(target: string) {
		this.recentTargets = this.recentTargets.filter((t) => t !== target);
		this.save();
	}
}

export const prefs = new Prefs();
