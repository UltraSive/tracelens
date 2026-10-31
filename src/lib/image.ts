import type { TraceSnapshot } from "./types";
import { ownerName } from "./format";

/**
 * Render a trace as a PNG for sharing. Drawn directly on a canvas (rather than screenshotting
 * the DOM) so the result is identical on every platform and always uses a light theme.
 */
export async function renderTraceImage(snap: TraceSnapshot): Promise<Uint8Array> {
	const scale = 2;
	const pad = 20;
	const rowH = 22;
	const font = "13px ui-sans-serif, system-ui, sans-serif";
	const mono = "12px ui-monospace, SFMono-Regular, Menlo, monospace";
	const cols = [
		{ key: "#", w: 30, right: true },
		{ key: "Host", w: 300 },
		{ key: "Network", w: 170 },
		{ key: "Loss", w: 60, right: true },
		{ key: "Sent", w: 50, right: true },
		{ key: "Avg", w: 60, right: true },
		{ key: "Best", w: 60, right: true },
		{ key: "Worst", w: 60, right: true },
		{ key: "Latency", w: 180 }
	];
	const tableW = cols.reduce((n, c) => n + c.w + 10, 0);
	const width = tableW + pad * 2;
	const details = snap.verdict.details;
	const headerH = 58 + 18 * details.length + 12;
	const height = headerH + rowH * (snap.hops.length + 1) + pad + 20;

	const canvas = document.createElement("canvas");
	canvas.width = width * scale;
	canvas.height = height * scale;
	const ctx = canvas.getContext("2d")!;
	ctx.scale(scale, scale);
	ctx.fillStyle = "#ffffff";
	ctx.fillRect(0, 0, width, height);

	const levelColor = { pending: "#71717a", ok: "#059669", info: "#0284c7", warn: "#d97706", bad: "#dc2626" }[snap.verdict.level];

	ctx.fillStyle = "#18181b";
	ctx.font = `600 15px ui-sans-serif, system-ui, sans-serif`;
	ctx.fillText(`Trace to ${snap.target} (${snap.targetAddr})`, pad, pad + 12);
	ctx.font = "12px ui-sans-serif, system-ui, sans-serif";
	ctx.fillStyle = "#71717a";
	ctx.fillText(
		`${snap.options.protocol.toUpperCase()} · ${snap.rounds} rounds · ${new Date(snap.startedAt).toLocaleString()}`,
		pad,
		pad + 30
	);
	ctx.fillStyle = levelColor;
	ctx.fillRect(pad, pad + 40, 3, 18 + 18 * details.length);
	ctx.font = `600 ${font}`;
	ctx.fillText(snap.verdict.headline, pad + 10, pad + 54);
	ctx.font = "12px ui-sans-serif, system-ui, sans-serif";
	ctx.fillStyle = "#52525b";
	details.forEach((d, i) => ctx.fillText(`• ${d}`, pad + 10, pad + 72 + i * 18, tableW - 20));

	let y = headerH + pad;
	ctx.fillStyle = "#f4f4f5";
	ctx.fillRect(pad, y - 15, tableW, rowH);
	ctx.font = "600 11px ui-sans-serif, system-ui, sans-serif";
	ctx.fillStyle = "#71717a";
	let x = pad + 5;
	for (const c of cols) {
		ctx.textAlign = c.right ? "right" : "left";
		ctx.fillText(c.key.toUpperCase(), c.right ? x + c.w : x, y);
		x += c.w + 10;
	}

	const all = snap.hops.flatMap((h) => h.samples.filter((s): s is number => s != null)).sort((a, b) => a - b);
	const max = all.length ? Math.max(1, all[Math.floor(all.length * 0.98)] * 1.1) : 1;

	for (const h of snap.hops) {
		y += rowH;
		ctx.strokeStyle = "#e4e4e7";
		ctx.beginPath();
		ctx.moveTo(pad, y - 15);
		ctx.lineTo(pad + tableW, y - 15);
		ctx.stroke();
		const host = h.hosts[0];
		const lossColor = h.sent === 0 ? "#a1a1aa" : h.lossPct === 0 ? "#059669" : h.lossPct < 5 ? "#d97706" : "#dc2626";
		const cells: [string, string?, string?][] = [
			[String(h.ttl), "#71717a"],
			[host ? (host.hostname ? `${host.hostname} (${host.ip})` : host.ip) + (h.hosts.length > 1 ? ` +${h.hosts.length - 1}` : "") : "???", undefined, font],
			[ownerName(h) ?? "", "#52525b", font],
			[h.sent ? `${h.lossPct.toFixed(1)}%` : "–", lossColor],
			[String(h.sent)],
			[h.recv ? h.avgMs.toFixed(1) : "–"],
			[h.bestMs?.toFixed(1) ?? "–"],
			[h.worstMs?.toFixed(1) ?? "–"]
		];
		x = pad + 5;
		cells.forEach(([text, color, f], i) => {
			const c = cols[i];
			ctx.font = f ?? mono;
			ctx.fillStyle = color ?? "#18181b";
			ctx.textAlign = c.right ? "right" : "left";
			ctx.fillText(text, c.right ? x + c.w : x, y, c.w);
			x += c.w + 10;
		});
		// sparkline
		const n = 60;
		const recent = h.samples.slice(-n);
		const step = cols.at(-1)!.w / n;
		const x0 = x + (n - recent.length) * step;
		const top = y - 13;
		const hgt = 14;
		let prev: [number, number] | null = null;
		recent.forEach((v, i) => {
			const px = x0 + i * step + step / 2;
			if (v == null) {
				ctx.fillStyle = "rgba(220,38,38,0.6)";
				ctx.fillRect(px - step / 2, top, Math.max(1, step - 0.5), hgt);
				prev = null;
				return;
			}
			const py = top + hgt - (Math.min(v, max) / max) * hgt;
			ctx.strokeStyle = "#0ea5e9";
			ctx.lineWidth = 1.25;
			if (prev) {
				ctx.beginPath();
				ctx.moveTo(prev[0], prev[1]);
				ctx.lineTo(px, py);
				ctx.stroke();
			}
			prev = [px, py];
		});
	}

	ctx.textAlign = "right";
	ctx.font = "10px ui-sans-serif, system-ui, sans-serif";
	ctx.fillStyle = "#a1a1aa";
	ctx.fillText("TraceLens", width - pad, height - 8);

	const blob = await new Promise<Blob>((res, rej) => canvas.toBlob((b) => (b ? res(b) : rej(new Error("render failed"))), "image/png"));
	return new Uint8Array(await blob.arrayBuffer());
}
