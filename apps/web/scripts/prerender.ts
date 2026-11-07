/**
 * Static pages for the landing page (design concept §5): `/`, `/de/` and `/404.html`,
 * rendered with react-dom/server through Vite's SSR loader. In dev the same renderer answers
 * requests; after `vite build` it writes the pages next to the hashed assets, adds the
 * `_headers` file, and fails the build on any external origin in the output.
 */
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { createServer, type Manifest, type Plugin, type ViteDevServer } from "vite";
import type { PageAssets, PageRequest } from "../src/pages/render.tsx";

const RENDER = "/src/pages/render.tsx";
const ENTRY = "src/main.ts";
const DEV_TOKENS = "/__mosaic-tokens.css";

interface RenderModule {
  renderPage: (request: PageRequest) => string;
}

const PAGES = [
  { url: "/", file: "index.html", locale: "en", page: "home" },
  { url: "/de/", file: "de/index.html", locale: "de", page: "home" },
  { url: "/404.html", file: "404.html", locale: "en", page: "notFound" },
] as const;

async function load(server: ViteDevServer): Promise<RenderModule> {
  return (await server.ssrLoadModule(RENDER)) as RenderModule;
}

function route(url: string): (typeof PAGES)[number] | undefined {
  const path = url.split("?")[0] ?? "/";
  const normal = path === "/index.html" ? "/" : path === "/de" || path === "/de/index.html" ? "/de/" : path;
  return PAGES.find((page) => page.url === normal);
}

/** Origins that may appear in the output: SVG namespaces and the placeholder domain in rel links. */
function allowedOrigins(domain: string): readonly string[] {
  return ["http://www.w3.org", `https://${domain}`];
}

function walk(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    return entry.isDirectory() ? walk(path) : [path];
  });
}

/**
 * Throws when a built file the browser parses (HTML, CSS, JS, JSON, SVG, headers) names an
 * origin outside the allow list (concept §5 Privacy). The OFL texts are plain documents.
 */
export function checkOrigins(outDir: string, domain: string): void {
  const allowed = allowedOrigins(domain);
  const offenders: string[] = [];
  for (const file of walk(outDir).filter((f) => /\.(html|css|js|json|svg)$/.test(f) || f.endsWith("_headers"))) {
    const text = readFileSync(file, "utf8");
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
    modules: (entry.dynamicImports ?? []).flatMap((key) => {
      const chunk = manifest[key];
      return chunk === undefined ? [] : [`/${chunk.file}`];
    }),
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
    for (const page of PAGES) {
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
        const page = route(req.url ?? "/");
        if (page === undefined) {
          next();
          return;
        }
        const assets: PageAssets = { styles: [DEV_TOKENS, "/src/styles/index.css"], scripts: [`/${ENTRY}`], modules: [], fonts: [], csp: false };
        load(server)
          .then(({ renderPage }) => server.transformIndexHtml(page.url, renderPage({ locale: page.locale, page: page.page, assets })))
          .then((html) => {
            res.statusCode = page.page === "notFound" ? 404 : 200;
            res.setHeader("Content-Type", "text/html; charset=utf-8");
            res.end(html);
          })
          .catch(next);
      });
    },
    buildEnd(error) {
      failed = error !== undefined;
    },
    generateBundle() {
      this.emitFile({ type: "asset", fileName: "_headers", source: options.headers });
    },
    async closeBundle() {
      if (failed) {
        return;
      }
      await writePages(root, outDir, options.ssrPlugins);
      checkOrigins(outDir, options.domain);
    },
  };
}
