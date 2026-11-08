/**
 * Static pages of the website (website v2 §1.4): every route of `src/pages/routes.ts` in
 * English and German at `<path>/index.html`, plus the bilingual `/404.html`, rendered with
 * react-dom/server through Vite's SSR loader. In dev the same renderer answers requests. After
 * `vite build` it writes the pages next to the hashed assets, adds `_headers`, `_redirects`,
 * `sitemap.xml` and `robots.txt`, and fails the build on an external origin, a broken internal
 * link, an unverified comparison cell, or missing legal texts in a production build.
 */
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { createServer, type Manifest, type Plugin, type ViteDevServer } from "vite";
import type { PageAssets, PageName, PageRequest } from "../src/pages/render.tsx";
import { fileOf, lookup, NOT_FOUND_PATH, ROUTES } from "../src/pages/routes.ts";

const RENDER = "/src/pages/render.tsx";
const ENTRY = "src/main.ts";
const DEV_TOKENS = "/__mosaic-tokens.css";

interface RenderModule {
  renderPage: (request: PageRequest) => string;
}

interface OutPage {
  file: string;
  locale: "en" | "de";
  page: PageName;
}

/** Every page the build writes. */
export function outPages(): OutPage[] {
  const pages: OutPage[] = ROUTES.flatMap((r) => [
    { file: fileOf(r.en), locale: "en" as const, page: r.id },
    { file: fileOf(r.de), locale: "de" as const, page: r.id },
  ]);
  pages.push({ file: NOT_FOUND_PATH.slice(1), locale: "en", page: "notFound" });
  return pages;
}

async function load(server: ViteDevServer): Promise<RenderModule> {
  return (await server.ssrLoadModule(RENDER)) as RenderModule;
}

function walk(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    return entry.isDirectory() ? walk(path) : [path];
  });
}

/**
 * Throws when a built file the browser parses (HTML, CSS, JS, JSON, SVG, XML, headers) names
 * an origin outside the allow list (concept §5 Privacy). The only exception: `<a href>` links
 * in HTML to the comparison's sources (`sources`), which are navigation, not requests.
 */
export function checkOrigins(outDir: string, domain: string, sources: readonly string[]): void {
  const allowed = ["http://www.w3.org", "http://www.sitemaps.org", "https://schema.org", `https://${domain}`];
  const offenders: string[] = [];
  const files = walk(outDir).filter((f) => /\.(html|css|js|json|svg|xml|txt)$/.test(f) || /_(headers|redirects)$/.test(f));
  for (const file of files.filter((f) => !f.includes(join("licenses", "")))) {
    let text = readFileSync(file, "utf8");
    if (file.endsWith(".html")) {
      text = text.replace(/<a [^>]*href="([^"]+)"/g, (tag, href: string) => (sources.includes(href.replace(/&amp;/g, "&")) ? "<a" : tag));
    }
    for (const match of text.matchAll(/(?:https?:)?\/\/[a-z0-9.-]+\.[a-z]{2,}/gi)) {
      const origin = match[0].startsWith("//") ? `https:${match[0]}` : match[0];
      if (!allowed.some((ok) => origin.toLowerCase().startsWith(ok))) {
        offenders.push(`${relative(outDir, file)}: ${match[0]}`);
      }
    }
  }
  if (offenders.length > 0) {
    throw new Error(`External origins in the build output:\n${offenders.join("\n")}`);
  }
}

/** The ids a page declares, for fragment checks. */
function idsOf(html: string): Set<string> {
  return new Set([...html.matchAll(/\sid="([^"]+)"/g)].map((m) => m[1] ?? ""));
}

/**
 * Throws when an internal link in a built page does not resolve: the path must be a written
 * file (`/x/` → `x/index.html`), and a `#fragment` must name an id on the target page.
 */
export function checkLinks(outDir: string): void {
  const broken: string[] = [];
  const pages = walk(outDir).filter((f) => f.endsWith(".html"));
  const cache = new Map<string, Set<string>>();
  const ids = (file: string): Set<string> => {
    let found = cache.get(file);
    if (found === undefined) {
      found = idsOf(readFileSync(file, "utf8"));
      cache.set(file, found);
    }
    return found;
  };
  for (const page of pages) {
    const html = readFileSync(page, "utf8");
    for (const [, raw = ""] of html.matchAll(/\s(?:href|src|srcSet|srcset)="([^"]+)"/g)) {
      for (const candidate of raw.split(/,\s*/)) {
        const href = candidate.trim().split(/\s+/)[0] ?? "";
        if (!href.startsWith("/") && !href.startsWith("#")) {
          continue;
        }
        const [path = "", hash] = href.split("#");
        const target = path === "" ? page : join(outDir, path.endsWith("/") ? `${path}index.html` : path.split("?")[0] ?? "");
        if (!existsSync(target)) {
          broken.push(`${relative(outDir, page)}: ${href}`);
        } else if (hash !== undefined && hash !== "" && target.endsWith(".html") && !ids(target).has(hash)) {
          broken.push(`${relative(outDir, page)}: ${href} (no #${hash})`);
        }
      }
    }
  }
  if (broken.length > 0) {
    throw new Error(`Broken internal links in the build output:\n${broken.join("\n")}`);
  }
}

/** `sitemap.xml`: indexable routes in both languages with their alternates. */
export function sitemap(domain: string): string {
  const origin = `https://${domain}`;
  const urls = ROUTES.filter((r) => r.index).flatMap((r) =>
    [r.en, r.de].map(
      (loc) =>
        `  <url>\n    <loc>${origin}${loc}</loc>\n` +
        `    <xhtml:link rel="alternate" hreflang="en" href="${origin}${r.en}"/>\n` +
        `    <xhtml:link rel="alternate" hreflang="de" href="${origin}${r.de}"/>\n` +
        `    <xhtml:link rel="alternate" hreflang="x-default" href="${origin}${r.en}"/>\n  </url>`,
    ),
  );
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">',
    ...urls,
    "</urlset>",
    "",
  ].join("\n");
}

/** `_redirects`: slash-less paths to the slashed page (301). */
export function redirects(): string {
  const lines = ROUTES.flatMap((r) => [r.en, r.de])
    .filter((path) => path !== "/")
    .map((path) => `${path.replace(/\/$/, "")} ${path} 301`);
  return [...lines, ""].join("\n");
}

function robots(domain: string): string {
  return `User-agent: *\nAllow: /\n\nSitemap: https://${domain}/sitemap.xml\n`;
}

function buildAssets(outDir: string): PageAssets {
  const manifest = JSON.parse(readFileSync(join(outDir, ".vite/manifest.json"), "utf8")) as Manifest;
  const entry = manifest[ENTRY];
  if (entry === undefined) {
    throw new Error(`prerender: ${ENTRY} is missing from the Vite manifest`);
  }
  const font = readdirSync(join(outDir, "assets")).find((f) => /^dm-sans-latin-opsz-normal-.*\.woff2$/.test(f));
  return {
    styles: (entry.css ?? []).map((css) => `/${css}`),
    scripts: [`/${entry.file}`],
    fonts: font === undefined ? [] : [`/assets/${font}`],
    csp: true,
  };
}

async function writePages(root: string, outDir: string, plugins: Plugin[]): Promise<void> {
  const assets = buildAssets(outDir);
  const server = await createServer({
    root,
    configFile: false,
    logLevel: "error",
    plugins,
    server: { middlewareMode: true, hmr: false, ws: false },
    appType: "custom",
  });
  try {
    const { renderPage } = await load(server);
    for (const page of outPages()) {
      const target = join(outDir, page.file);
      mkdirSync(dirname(target), { recursive: true });
      writeFileSync(target, renderPage({ locale: page.locale, page: page.page, assets }));
    }
  } finally {
    await server.close();
  }
}

/**
 * The prerender plugin. `ssrPlugins` are the plugins the page renderer needs (the tokens
 * stylesheet), passed to the private SSR server used after the build.
 */
export function prerender(options: {
  ssrPlugins: Plugin[];
  domain: string;
  headers: string;
  /** External `<a href>` targets allowed in pages: the comparison's sources. */
  sources: readonly string[];
  /** The production build refuses to ship without the owner's legal texts. */
  legalReady: boolean;
  /** The design tokens stylesheet, linked as a real file in dev so the page never paints unstyled. */
  tokensCss: () => string;
}): Plugin {
  let root = process.cwd();
  let outDir = "dist";
  let failed = false;
  return {
    name: "mosaic-web-prerender",
    configResolved(config) {
      root = config.root;
      outDir = join(config.root, config.build.outDir);
    },
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (req.url === DEV_TOKENS) {
          res.setHeader("Content-Type", "text/css; charset=utf-8");
          res.end(options.tokensCss());
          return;
        }
        const url = req.url ?? "/";
        const found = lookup(url);
        const notFound = url.split("?")[0] === NOT_FOUND_PATH;
        if (found === undefined && !notFound) {
          next();
          return;
        }
        const assets: PageAssets = { styles: [DEV_TOKENS, "/src/styles/index.css"], scripts: [`/${ENTRY}`], fonts: [], csp: false };
        const request: PageRequest =
          found === undefined ? { locale: "en", page: "notFound", assets } : { locale: found.locale, page: found.route.id, assets };
        load(server)
          .then(({ renderPage }) => server.transformIndexHtml(url, renderPage(request)))
          .then((html) => {
            res.statusCode = notFound ? 404 : 200;
            res.setHeader("Content-Type", "text/html; charset=utf-8");
            res.end(html);
          })
          .catch(next);
      });
    },
    buildStart() {
      if (process.env["MOSAIC_DEPLOY"] === "production" && !options.legalReady) {
        throw new Error("prerender: the Imprint and Privacy texts are missing (site.legalReady); production builds stop here.");
      }
    },
    buildEnd(error) {
      failed = error !== undefined;
    },
    generateBundle() {
      this.emitFile({ type: "asset", fileName: "_headers", source: options.headers });
      this.emitFile({ type: "asset", fileName: "_redirects", source: redirects() });
      this.emitFile({ type: "asset", fileName: "sitemap.xml", source: sitemap(options.domain) });
      this.emitFile({ type: "asset", fileName: "robots.txt", source: robots(options.domain) });
    },
    async closeBundle() {
      if (failed) {
        return;
      }
      await writePages(root, outDir, options.ssrPlugins);
      checkOrigins(outDir, options.domain, options.sources);
      checkLinks(outDir);
      for (const page of outPages()) {
        if (/\bn\/v\b/.test(readFileSync(join(outDir, page.file), "utf8"))) {
          throw new Error(`prerender: ${page.file} still shows an unverified (n/v) comparison cell`);
        }
      }
    },
  };
}
