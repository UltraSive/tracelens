import { save } from "@tauri-apps/plugin-dialog";
import { writeFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { toast } from "svelte-sonner";
import { store, type Tab } from "./traces.svelte";
import { renderTraceImage } from "./image";
import type { ExportFormat } from "./types";

const LABEL: Record<ExportFormat, string> = { text: "report", markdown: "Markdown", csv: "CSV", json: "JSON" };
const EXT: Record<ExportFormat, string> = { text: "txt", markdown: "md", csv: "csv", json: "json" };

function baseName(tab: Tab) {
	const d = new Date(tab.snap.startedAt);
	const stamp = `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, "0")}${String(d.getDate()).padStart(2, "0")}-${String(d.getHours()).padStart(2, "0")}${String(d.getMinutes()).padStart(2, "0")}`;
	return `trace-${tab.snap.target.replace(/[^\w.-]+/g, "_")}-${stamp}`;
}

export async function copyExport(tab: Tab, format: ExportFormat) {
	try {
		await navigator.clipboard.writeText(await store.render(tab, format));
		toast.success(`Copied ${LABEL[format]} to clipboard`);
	} catch (e) {
		toast.error(`Copy failed: ${e}`);
	}
}

export async function saveExport(tab: Tab, format: ExportFormat) {
	const path = await save({
		defaultPath: `${baseName(tab)}.${EXT[format]}`,
		filters: [{ name: LABEL[format], extensions: [EXT[format]] }]
	});
	if (!path) return;
	try {
		await writeTextFile(path, await store.render(tab, format));
		toast.success(`Saved ${LABEL[format]}`, { description: path });
	} catch (e) {
		toast.error(`Save failed: ${e}`);
	}
}

export async function saveImage(tab: Tab) {
	const path = await save({ defaultPath: `${baseName(tab)}.png`, filters: [{ name: "PNG image", extensions: ["png"] }] });
	if (!path) return;
	try {
		await writeFile(path, await renderTraceImage(await store.fullSnapshot(tab)));
		toast.success("Saved image", { description: path });
	} catch (e) {
		toast.error(`Save failed: ${e}`);
	}
}

export async function saveTrace(tab: Tab) {
	try {
		const meta = await store.save(tab);
		if (meta) toast.success("Trace saved", { description: "Open it again from Saved traces." });
	} catch (e) {
		toast.error(`Save failed: ${e}`);
	}
}
