import { component$, useSignal } from '@builder.io/qwik';
import type { DocumentHead } from '@builder.io/qwik-city';

const INSTALL_CMD =
  'curl -fsSL https://raw.githubusercontent.com/tjklint/todork/main/install.sh | sh';

const GH = 'https://github.com/tjklint/todork';

// ── GitHub icon SVG ───────────────────────────────────────────────────────────
const GhIcon = () => (
  <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
    <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
  </svg>
);

// ── Film / GIF placeholder icon ───────────────────────────────────────────────
const FilmIcon = () => (
  <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor"
       stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
    <rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18" />
    <line x1="7" y1="2" x2="7" y2="22" />
    <line x1="17" y1="2" x2="17" y2="22" />
    <line x1="2" y1="12" x2="22" y2="12" />
    <line x1="2" y1="7" x2="7" y2="7" />
    <line x1="2" y1="17" x2="7" y2="17" />
    <line x1="17" y1="17" x2="22" y2="17" />
    <line x1="17" y1="7" x2="22" y2="7" />
  </svg>
);

export default component$(() => {
  const copied = useSignal(false);

  return (
    <>
      {/* ════════════════════════════════════════════════════════════
          NAV
      ══════════════════════════════════════════════════════════════ */}
      <nav class="nav">
        <div class="container">
          <div class="nav-inner">
            <a class="nav-logo" href="/todork/" style="text-decoration:none">
              <span class="logo-accent">✦</span> todork
            </a>
            <div>
              <a class="gh-btn" href={GH} target="_blank" rel="noopener noreferrer">
                <GhIcon />
                GitHub
              </a>
            </div>
          </div>
        </div>
      </nav>

      {/* ════════════════════════════════════════════════════════════
          HERO
      ══════════════════════════════════════════════════════════════ */}
      <section class="hero">
        <div class="container">
          <p class="hero-eyebrow">Rust · Open Source · MIT</p>

          <h1 class="hero-name">
            <span class="name-accent">to</span>dork
          </h1>

          <p class="hero-tagline">
            Hyper-fast annotation scanner for codebases.
            Find every TODO, FIXME, HACK and more&nbsp;— in milliseconds.
          </p>

          <div class="badge-row">
            <span class="badge badge-rust">🦀 Written in Rust</span>
            <span class="badge badge-oss">⭐ Open Source</span>
            <span class="badge badge-mit">📄 MIT Licensed</span>
          </div>

          {/* install command */}
          <div class="install-block">
            <div class="install-cmd">
              <span class="i-prompt">$ </span>
              {INSTALL_CMD}
            </div>
            <button
              class={`copy-btn${copied.value ? ' copied' : ''}`}
              title="Copy to clipboard"
              onClick$={async () => {
                await navigator.clipboard.writeText(INSTALL_CMD);
                copied.value = true;
                setTimeout(() => { copied.value = false; }, 2200);
              }}
            >
              {copied.value ? '✓ Copied!' : 'Copy'}
            </button>
          </div>
        </div>
      </section>

      <hr class="divider" />

      {/* ════════════════════════════════════════════════════════════
          TERMINAL DEMO  (--blame is the money shot)
      ══════════════════════════════════════════════════════════════ */}
      <section class="section">
        <div class="container">
          <p class="section-label center">demo</p>
          <h2 class="section-title center">See it in action</h2>

          <div class="terminal-wrap">
            {/* title bar */}
            <div class="terminal-bar">
              <span class="tdot tdot-r" />
              <span class="tdot tdot-y" />
              <span class="tdot tdot-g" />
              <span class="terminal-ttl">bash — ~/your-project</span>
            </div>

            {/* body */}
            <div class="terminal-body">
              {/* command */}
              <span class="tl">
                <span class="t-prompt">$ </span>
                <span class="t-cmd">todork ./src </span>
                <span class="t-flag">--blame</span>
              </span>
              <span class="tl-gap" />

              {/* ── result 1 ── */}
              <span class="tl">
                <span class="t-path">src/api/auth.rs</span>
                <span class="t-sep">:42:5: </span>
                <span class="t-todo">TODO</span>
                <span class="t-msg">: add rate limiting</span>
              </span>
              <span class="tl">
                <span class="t-tree">  └─ </span>
                <span class="t-author">alice &lt;alice@example.com&gt;</span>
                <span class="t-sep">  ·  </span>
                <span class="t-age">8 months ago</span>
                <span class="t-sep">  </span>
                <span class="t-hash">(a3f9c12)</span>
              </span>
              <span class="tl-gap" />

              {/* ── result 2 ── */}
              <span class="tl">
                <span class="t-path">src/db/queries.rs</span>
                <span class="t-sep">:17:3: </span>
                <span class="t-fixme">FIXME</span>
                <span class="t-msg">: N+1 query — fix before launch</span>
              </span>
              <span class="tl">
                <span class="t-tree">  └─ </span>
                <span class="t-author">bob &lt;bob@corp.dev&gt;</span>
                <span class="t-sep">  ·  </span>
                <span class="t-age">2 weeks ago</span>
                <span class="t-sep">  </span>
                <span class="t-hash">(f1a2b3c)</span>
              </span>
              <span class="tl-gap" />

              {/* ── result 3 ── */}
              <span class="tl">
                <span class="t-path">src/worker/job.rs</span>
                <span class="t-sep">:89:9: </span>
                <span class="t-hack">HACK</span>
                <span class="t-msg">: retry logic needs rework</span>
              </span>
              <span class="tl">
                <span class="t-tree">  └─ </span>
                <span class="t-author">alice &lt;alice@example.com&gt;</span>
                <span class="t-sep">  ·  </span>
                <span class="t-age">3 months ago</span>
                <span class="t-sep">  </span>
                <span class="t-hash">(d4e5f6a)</span>
              </span>
              <span class="tl-gap" />

              {/* ── summary ── */}
              <span class="tl t-summary">
                <span class="t-found">Found 3</span>{' '}
                annotation(s) in <span class="t-time">0.028s</span>
              </span>
            </div>
          </div>
        </div>
      </section>

      <hr class="divider" />

      {/* ════════════════════════════════════════════════════════════
          GIF PLACEHOLDERS
          TJ: drop your GIFs in website/public/ and swap the <div>s
          below for <img> tags, e.g.:
            <img src="/todork/scan-demo.gif" alt="generic scan demo" />
            <img src="/todork/blame-demo.gif" alt="blame mode demo" />
      ══════════════════════════════════════════════════════════════ */}
      <section class="section">
        <div class="container">
          <p class="section-label center">see it run</p>
          <h2 class="section-title center">In the wild</h2>

          <div class="gif-grid">
            {/* slot 1 — generic scan GIF */}
            <div class="gif-card">
              <div class="gif-frame">
                <FilmIcon />
                <span class="gif-frame-label">GIF coming soon</span>
              </div>
              <div class="gif-caption">Generic scan</div>
              <div class="gif-sub">todork ./your-project</div>
            </div>

            {/* slot 2 — blame mode GIF */}
            <div class="gif-card">
              <div class="gif-frame">
                <FilmIcon />
                <span class="gif-frame-label">GIF coming soon</span>
              </div>
              <div class="gif-caption">Git blame enrichment</div>
              <div class="gif-sub">todork ./your-project --blame</div>
            </div>
          </div>
        </div>
      </section>

      <hr class="divider" />

      {/* ════════════════════════════════════════════════════════════
          FEATURES
      ══════════════════════════════════════════════════════════════ */}
      <section class="section">
        <div class="container">
          <p class="section-label center">why todork</p>
          <h2 class="section-title center">Built for speed &amp; signal</h2>

          <div class="features-grid">
            <div class="feat-card">
              <span class="feat-icon">⚡</span>
              <div>
                <div class="feat-name">Ripgrep-class speed</div>
                <div class="feat-desc">
                  Parallel walks with <code>ignore::WalkParallel</code> and
                  multi-pattern AhoCorasick matching — scans thousands of files
                  in under a second.
                </div>
              </div>
            </div>

            <div class="feat-card">
              <span class="feat-icon">🦀</span>
              <div>
                <div class="feat-name">Written in Rust</div>
                <div class="feat-desc">
                  Zero-overhead abstractions, memory-safe by default, no
                  runtime or interpreter required.
                </div>
              </div>
            </div>

            <div class="feat-card">
              <span class="feat-icon">🔍</span>
              <div>
                <div class="feat-name">8 annotation types</div>
                <div class="feat-desc">
                  TODO, FIXME, HACK, NOTE, BUG, OPTIMIZE, REVIEW, XXX —
                  all configurable with <code>--tags</code>.
                </div>
              </div>
            </div>

            <div class="feat-card">
              <span class="feat-icon">📋</span>
              <div>
                <div class="feat-name">Multiple output formats</div>
                <div class="feat-desc">
                  Human-readable text, machine-parseable JSON, and native
                  GitHub Annotations for CI pipelines.
                </div>
              </div>
            </div>

            <div class="feat-card">
              <span class="feat-icon">🔀</span>
              <div>
                <div class="feat-name">Git blame enrichment</div>
                <div class="feat-desc">
                  Add <code>--blame</code> to see exactly who wrote each
                  annotation, when, and which commit introduced it.
                </div>
              </div>
            </div>

            <div class="feat-card">
              <span class="feat-icon">⚙️</span>
              <div>
                <div class="feat-name">Fully configurable</div>
                <div class="feat-desc">
                  Glob include/exclude, max depth, filesize limits, thread
                  count, gitignore awareness, hidden files — all flags.
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <hr class="divider" />

      {/* ════════════════════════════════════════════════════════════
          QUICK START
      ══════════════════════════════════════════════════════════════ */}
      <section class="section">
        <div class="container">
          <p class="section-label center">get going</p>
          <h2 class="section-title center">Quick start</h2>

          <div class="qs-grid">
            <div class="qs-card">
              <div class="qs-num">01 — INSTALL</div>
              <div class="qs-code">
                <span class="q-prompt">$ </span>
                curl -fsSL …/install.sh | sh
              </div>
              <div class="qs-desc">One-liner installer. Detects your OS &amp; arch, verifies the SHA-256 checksum, drops the binary in <code>~/.local/bin</code>.</div>
            </div>

            <div class="qs-card">
              <div class="qs-num">02 — SCAN</div>
              <div class="qs-code">
                <span class="q-prompt">$ </span>
                todork ./src
              </div>
              <div class="qs-desc">Scans the directory recursively, respects <code>.gitignore</code>, and prints every annotation found.</div>
            </div>

            <div class="qs-card">
              <div class="qs-num">03 — BLAME</div>
              <div class="qs-code">
                <span class="q-prompt">$ </span>
                todork . --blame
              </div>
              <div class="qs-desc">Enriches every result with author, age, and commit hash via <code>git blame</code>.</div>
            </div>

            <div class="qs-card">
              <div class="qs-num">04 — CI</div>
              <div class="qs-code">
                <span class="q-prompt">$ </span>
                todork . --format github-annotations
              </div>
              <div class="qs-desc">Outputs native GitHub Annotations so results surface inline in pull-request diffs.</div>
            </div>
          </div>
        </div>
      </section>

      <hr class="divider" />

      {/* ════════════════════════════════════════════════════════════
          FOOTER
      ══════════════════════════════════════════════════════════════ */}
      <footer class="footer">
        <div class="container">
          <p class="footer-text">
            <a href={GH} target="_blank" rel="noopener noreferrer">
              tjklint/todork
            </a>
            <span class="fsep">·</span>
            MIT Licensed
            <span class="fsep">·</span>
            Built with 🦀 Rust
            <span class="fsep">·</span>
            <a href={`${GH}/releases`} target="_blank" rel="noopener noreferrer">
              Releases
            </a>
            <span class="fsep">·</span>
            <a href={`${GH}/issues`} target="_blank" rel="noopener noreferrer">
              Issues
            </a>
          </p>
        </div>
      </footer>
    </>
  );
});

export const head: DocumentHead = {
  title: 'todork — hyper-fast TODO scanner',
  meta: [
    {
      name: 'description',
      content:
        'Scan your entire codebase for TODO, FIXME, HACK and more in milliseconds. Written in Rust. Open source. MIT licensed.',
    },
    { property: 'og:title',       content: 'todork' },
    { property: 'og:description', content: 'Hyper-fast annotation scanner for codebases. Written in Rust.' },
    { property: 'og:type',        content: 'website' },
    { name: 'theme-color',        content: '#0a0e1a' },
    { name: 'twitter:card',       content: 'summary' },
  ],
};
