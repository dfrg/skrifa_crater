// Renders runs/index.json as a list of runs, and one run's results as a
// grid of fonts and configurations with drawings of the mismatches.

const app = document.getElementById("app");

function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (key === "class") node.className = value;
    else node.setAttribute(key, value);
  }
  for (const child of children.flat()) {
    if (child == null) continue;
    node.append(child instanceof Node ? child : document.createTextNode(String(child)));
  }
  return node;
}

function pct(part, whole) {
  return whole ? (100 * part / whole).toFixed(part === whole ? 0 : 2) + "%" : "–";
}

function status(totals, failed) {
  if (failed) return el("span", { class: "badge bad" }, "build failed");
  const bad = totals.mismatch + totals.advance + totals.errors;
  if (bad) return el("span", { class: "badge bad" }, `${bad} differ`);
  if (totals.off_by_one) return el("span", { class: "badge warn" }, `${totals.off_by_one} off by one`);
  return el("span", { class: "badge ok" }, "match");
}

function commitLink(meta) {
  const repo = meta.repo.replace(/\.git$/, "");
  return el("a", { href: `${repo}/commit/${meta.sha}`, class: "mono" }, meta.sha.slice(0, 8));
}

async function loadJson(path) {
  const response = await fetch(path, { cache: "no-cache" });
  if (!response.ok) throw new Error(`${path}: ${response.status}`);
  return response.json();
}

async function showIndex() {
  let runs;
  try {
    runs = await loadJson("runs/index.json");
  } catch {
    app.replaceChildren(el("p", { class: "muted" }, "No runs yet."));
    return;
  }
  const rows = runs.map((run) =>
    el("tr", {},
      el("td", {}, el("a", { href: `#run=${encodeURIComponent(run.file)}` }, run.meta.date.replace("T", " ").replace("Z", ""))),
      el("td", {}, el("code", {}, run.meta.ref)),
      el("td", {}, commitLink(run.meta)),
      el("td", {}, status(run.totals, run.build_failed)),
      el("td", { class: "num" }, run.totals.glyphs),
      el("td", { class: "num" }, pct(run.totals.match, run.totals.glyphs)),
      el("td", { class: "mono" }, run.meta.freetype)));
  app.replaceChildren(
    el("h2", {}, "Runs"),
    el("div", { class: "scroll" },
      el("table", {},
        el("thead", {}, el("tr", {},
          el("th", {}, "Date (UTC)"), el("th", {}, "Ref"), el("th", {}, "Commit"),
          el("th", {}, "Status"), el("th", { class: "num" }, "Glyphs"),
          el("th", { class: "num" }, "Exact"), el("th", {}, "FreeType"))),
        el("tbody", {}, rows))));
}

function cellFor(config) {
  const bad = config.glyphs - config.outlines_match - config.outlines_off_by_one
    + config.advance_mismatches + config.errors_skrifa + config.missing;
  const cls = bad ? "bad" : config.outlines_off_by_one ? "warn" : "ok";
  const text = bad ? `${bad} differ` : config.outlines_off_by_one ? `${config.outlines_off_by_one} ±1` : "match";
  return el("span", { class: `cell ${cls}`, title: `${config.glyphs} glyphs` }, text);
}

// Parses a record path into commands with numeric arguments.
function parsePath(text) {
  const tokens = text.trim().split(/\s+/).filter(Boolean);
  const counts = { M: 2, L: 2, Q: 4, C: 6, Z: 0 };
  const commands = [];
  for (let i = 0; i < tokens.length;) {
    const op = tokens[i];
    const n = counts[op];
    if (n === undefined) return [];
    commands.push([op, tokens.slice(i + 1, i + 1 + n).map(Number)]);
    i += 1 + n;
  }
  return commands;
}

function svgPath(commands) {
  let d = "";
  for (const [op, args] of commands) {
    const pts = [];
    for (let i = 0; i < args.length; i += 2) pts.push(`${args[i]} ${-args[i + 1]}`);
    d += op + pts.join(" ");
  }
  return d;
}

function bounds(commandLists) {
  let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
  for (const commands of commandLists)
    for (const [, args] of commands)
      for (let i = 0; i < args.length; i += 2) {
        x0 = Math.min(x0, args[i]); x1 = Math.max(x1, args[i]);
        y0 = Math.min(y0, -args[i + 1]); y1 = Math.max(y1, -args[i + 1]);
      }
  return Number.isFinite(x0) ? [x0, y0, x1, y1] : [0, 0, 1, 1];
}

function drawing(example) {
  const oracle = parsePath(example.oracle);
  const skrifa = parsePath(example.skrifa);
  const [x0, y0, x1, y1] = bounds([oracle, skrifa]);
  const pad = Math.max(x1 - x0, y1 - y0) * 0.05 + 1;
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", `${x0 - pad} ${y0 - pad} ${x1 - x0 + 2 * pad} ${y1 - y0 + 2 * pad}`);
  for (const [commands, color] of [[oracle, "var(--oracle)"], [skrifa, "var(--skrifa)"]]) {
    const path = document.createElementNS(ns, "path");
    path.setAttribute("d", svgPath(commands));
    path.setAttribute("fill", "none");
    path.setAttribute("vector-effect", "non-scaling-stroke");
    // Presentation attributes don't resolve CSS variables; styles do.
    path.style.stroke = color;
    path.style.strokeWidth = "1.5px";
    svg.append(path);
  }
  return svg;
}

function exampleCard(example) {
  const isPath = example.kind.startsWith("outline");
  return el("div", { class: "example" },
    el("div", { class: "label" }, el("strong", {}, `glyph ${example.gid}`), " · ", example.kind),
    isPath ? drawing(example)
      : el("div", { class: "mono" }, `FreeType: ${example.oracle}`, el("br"), `skrifa: ${example.skrifa}`));
}

async function showRun(file, focus) {
  const run = await loadJson(`runs/${file}`);
  const meta = run.meta;
  const parts = [
    el("p", {}, el("a", { href: "#" }, "← All runs")),
    el("h2", {}, el("code", {}, meta.ref), " at ", commitLink(meta)),
    el("p", { class: "muted" }, `${meta.date} · FreeType ${meta.freetype}`),
  ];
  if (run.build_failed) {
    parts.push(el("p", {}, el("span", { class: "badge bad" }, "build failed"),
      " The harness did not build against this ref."), el("pre", { class: "log" }, run.log));
    app.replaceChildren(...parts);
    return;
  }
  const names = [...new Set(run.fonts.flatMap((f) => f.configs.map((c) => c.name)))];
  const rows = run.fonts.map((font) =>
    el("tr", {},
      el("td", {}, el("a", { href: `#run=${encodeURIComponent(file)}&font=${encodeURIComponent(font.id)}` }, font.id)),
      names.map((name) => {
        const config = font.configs.find((c) => c.name === name);
        return el("td", {}, config ? cellFor(config) : "–");
      })));
  parts.push(el("div", { class: "scroll" },
    el("table", {},
      el("thead", {}, el("tr", {}, el("th", {}, "Font"), names.map((n) => el("th", {}, n)))),
      el("tbody", {}, rows))));
  const font = run.fonts.find((f) => f.id === focus);
  if (font) {
    parts.push(el("h2", {}, font.id),
      el("p", { class: "legend" }, el("span", { class: "o" }, "■ FreeType"), el("span", { class: "s" }, "■ skrifa")));
    for (const config of font.configs) {
      if (!config.examples.length) continue;
      parts.push(el("h3", {}, config.name),
        el("div", { class: "examples" }, config.examples.map(exampleCard)));
    }
    if (!font.configs.some((c) => c.examples.length))
      parts.push(el("p", { class: "muted" }, "Every glyph matches."));
  }
  app.replaceChildren(...parts);
}

async function route() {
  const params = new URLSearchParams(location.hash.slice(1));
  try {
    if (params.get("run")) await showRun(params.get("run"), params.get("font"));
    else await showIndex();
  } catch (error) {
    app.replaceChildren(el("p", { class: "bad" }, String(error)));
  }
}

window.addEventListener("hashchange", route);
route();
