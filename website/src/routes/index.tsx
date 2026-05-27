import { component$, useSignal, useVisibleTask$ } from '@builder.io/qwik';
import type { DocumentHead } from '@builder.io/qwik-city';

const INSTALL_CMD =
  'curl -fsSL https://raw.githubusercontent.com/tjklint/todork/main/install.sh | sh';
const GH = 'https://github.com/tjklint/todork';

// ── terminal animation data ───────────────────────────────────────────────────

type CmdToken = { text: string; cls?: string };
type TermLine =
  | { t: 'finding'; path: string; loc: string; tag: string; cls: string; msg: string }
  | { t: 'blame';   author: string; age: string; hash: string }
  | { t: 'gap' }
  | { t: 'summary'; label: string; time: string };

interface Scene { tokens: CmdToken[]; lines: TermLine[] }

// Scene 1 - plain scan: establishes speed and simplicity as the baseline
// Scene 2 - --blame:   same three files, now author/age/hash appear
//            the contrast between the two is the "aha" moment
const SCENES: Scene[] = [
  {
    tokens: [{ text: 'todork' }, { text: ' ./src' }],
    lines: [
      { t: 'finding', path: 'src/api/auth.rs',   loc: ':42:5:', tag: 'TODO',  cls: 't-todo',  msg: 'add rate limiting' },
      { t: 'gap' },
      { t: 'finding', path: 'src/db/queries.rs', loc: ':17:3:', tag: 'FIXME', cls: 't-fixme', msg: 'N+1 query - fix before launch' },
      { t: 'gap' },
      { t: 'finding', path: 'src/worker/job.rs', loc: ':89:9:', tag: 'HACK',  cls: 't-hack',  msg: 'retry logic needs rework' },
      { t: 'gap' },
      { t: 'summary', label: 'Found 3 annotations across 3 files.', time: '0.009s' },
    ],
  },
  {
    tokens: [{ text: 'todork' }, { text: ' ./src' }, { text: ' --blame', cls: 't-flag' }],
    lines: [
      { t: 'finding', path: 'src/api/auth.rs',   loc: ':42:5:', tag: 'TODO',  cls: 't-todo',  msg: 'add rate limiting' },
      { t: 'blame',   author: 'alice <alice@example.com>', age: '8 months ago', hash: 'a3f9c12' },
      { t: 'gap' },
      { t: 'finding', path: 'src/db/queries.rs', loc: ':17:3:', tag: 'FIXME', cls: 't-fixme', msg: 'N+1 query - fix before launch' },
      { t: 'blame',   author: 'bob <bob@corp.dev>',          age: '2 weeks ago',  hash: 'f1a2b3c' },
      { t: 'gap' },
      { t: 'finding', path: 'src/worker/job.rs', loc: ':89:9:', tag: 'HACK',  cls: 't-hack',  msg: 'retry logic needs rework' },
      { t: 'blame',   author: 'alice <alice@example.com>', age: '3 months ago', hash: 'd4e5f6a' },
      { t: 'gap' },
      { t: 'summary', label: 'Found 3 annotations across 3 files.', time: '0.028s' },
    ],
  },
];

type Phase = 'typing' | 'running' | 'revealing' | 'paused' | 'clearing';

// ── icons ─────────────────────────────────────────────────────────────────────

const GhIcon = () => (
  <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
    <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
  </svg>
);

// ── page component ────────────────────────────────────────────────────────────

export default component$(() => {
  const copied = useSignal(false);

  // animation signals
  const sceneIdx  = useSignal(0);
  const phase     = useSignal<Phase>('typing');
  const typedLen  = useSignal(0);
  const revealedN = useSignal(0);

  useVisibleTask$(({ cleanup }) => {
    // Respect prefers-reduced-motion: jump straight to the blame scene, static
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      const s = SCENES[1];
      sceneIdx.value  = 1;
      typedLen.value  = s.tokens.reduce((a, t) => a + t.text.length, 0);
      revealedN.value = s.lines.length;
      phase.value     = 'paused';
      return;
    }

    let tid: ReturnType<typeof setTimeout>;

    const tick = () => {
      const scene  = SCENES[sceneIdx.value];
      const cmdLen = scene.tokens.reduce((a, t) => a + t.text.length, 0);

      switch (phase.value) {
        case 'typing':
          if (typedLen.value < cmdLen) {
            typedLen.value++;
            // small jitter per keystroke makes it feel human
            tid = setTimeout(tick, 55 + Math.random() * 40);
          } else {
            phase.value = 'running';
            tid = setTimeout(tick, 520);
          }
          break;

        case 'running':
          phase.value = 'revealing';
          tid = setTimeout(tick, 40);
          break;

        case 'revealing':
          if (revealedN.value < scene.lines.length) {
            revealedN.value++;
            const line = scene.lines[revealedN.value - 1];
            // blame + gap snap in fast - they feel grouped with the finding above
            const delay = line.t === 'gap' ? 25 : line.t === 'blame' ? 45 : 115;
            tid = setTimeout(tick, delay);
          } else {
            phase.value = 'paused';
            tid = setTimeout(tick, 4200);
          }
          break;

        case 'paused':
          phase.value = 'clearing';
          tid = setTimeout(tick, 40);
          break;

        case 'clearing':
          sceneIdx.value  = (sceneIdx.value + 1) % SCENES.length;
          typedLen.value  = 0;
          revealedN.value = 0;
          phase.value     = 'typing';
          tid = setTimeout(tick, 800);
          break;
      }
    };

    tid = setTimeout(tick, 1000);
    cleanup(() => clearTimeout(tid));
  });

  // ── reactive render helpers ───────────────────────────────────────────────

  const scene            = SCENES[sceneIdx.value];
  const cursorOnCmd      = phase.value === 'typing' || phase.value === 'running';
  const showOutput       = phase.value !== 'typing' && phase.value !== 'running';
  const showTrailCursor  = phase.value === 'paused' && revealedN.value >= scene.lines.length;

  // slice command string with per-token colour classes
  let rem = typedLen.value;
  const cmdNodes = scene.tokens.map((tok, i) => {
    if (rem <= 0) return null;
    const vis = tok.text.slice(0, Math.min(rem, tok.text.length));
    rem = Math.max(0, rem - tok.text.length);
    return vis ? <span key={i} class={tok.cls ?? 't-cmd'}>{vis}</span> : null;
  });

  return (
    <>
      {/* ══ NAV ══════════════════════════════════════════════════════════ */}
      <nav class="nav">
        <div class="container">
          <div class="nav-inner">
            <a class="nav-logo" href="/todork/" style="text-decoration:none">
              <span class="logo-mark">td</span> todork
            </a>
            <a class="gh-btn" href={GH} target="_blank" rel="noopener noreferrer">
              <GhIcon /> GitHub
            </a>
          </div>
        </div>
      </nav>

      {/* ══ HERO ═════════════════════════════════════════════════════════ */}
      <section class="hero">
        <div class="container">
          <p class="hero-eyebrow">Rust · Open Source · MIT</p>
          <h1 class="hero-name"><span class="name-accent">to</span>dork</h1>
          <p class="hero-pronunciation">/ ˈtuː.dɔːrk / <span class="pronunciation-plain"> TOO-dork</span></p>
          <p class="hero-tagline">
            Hyper-fast annotation scanner for codebases.{' '}
            Find every TODO, FIXME, HACK and more&nbsp;- in milliseconds.
          </p>

          <div class="badge-row">
            <span class="badge badge-rust"><span class="badge-dot" />Written in Rust</span>
            <span class="badge badge-oss"><span class="badge-dot" />Open Source</span>
            <span class="badge badge-mit"><span class="badge-dot" />MIT Licensed</span>
          </div>

          <div class="install-block">
            <div class="install-cmd">
              <span class="i-prompt">$ </span>{INSTALL_CMD}
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

          {/* ── animated terminal ────────────────────────────────────── */}
          <div class="hero-terminal">
            <div class="terminal-wrap">
              <div class="terminal-bar">
                <span class="tdot tdot-r" />
                <span class="tdot tdot-y" />
                <span class="tdot tdot-g" />
                <span class="terminal-ttl">bash - ~/your-project</span>
              </div>

              <div class="terminal-body" aria-live="polite" aria-label="Terminal demo">
                {/* command line */}
                <div class="tl">
                  <span class="t-prompt">$ </span>
                  {cmdNodes}
                  {cursorOnCmd && <span class="t-cursor" />}
                </div>

                {/* output lines - each fades in as it's revealed */}
                {showOutput && (
                  <>
                    <span class="tl-gap" />
                    {scene.lines.slice(0, revealedN.value).map((line, i) => {
                      if (line.t === 'gap') {
                        return <span key={i} class="tl-gap" />;
                      }
                      if (line.t === 'finding') {
                        return (
                          <div key={i} class="tl tl-in">
                            <span class="t-path">{line.path}</span>
                            <span class="t-sep">{line.loc} </span>
                            <span class={line.cls}>{line.tag}</span>
                            <span class="t-msg">: {line.msg}</span>
                          </div>
                        );
                      }
                      if (line.t === 'blame') {
                        return (
                          <div key={i} class="tl tl-in">
                            <span class="t-tree">  └─ </span>
                            <span class="t-author">{line.author}</span>
                            <span class="t-sep">  ·  </span>
                            <span class="t-age">{line.age}</span>
                            <span class="t-sep">  </span>
                            <span class="t-hash">({line.hash})</span>
                          </div>
                        );
                      }
                      if (line.t === 'summary') {
                        return (
                          <div key={i} class="tl tl-in">
                            <span class="t-found">{line.label}</span>
                            <span class="t-dim">  {line.time}</span>
                          </div>
                        );
                      }
                      return null;
                    })}

                    {/* trailing cursor once all output is revealed */}
                    {showTrailCursor && (
                      <div class="tl"><span class="t-cursor" /></div>
                    )}
                  </>
                )}
              </div>
            </div>
          </div>
        </div>
      </section>

      <hr class="divider" />

      {/* ══ GIF PLACEHOLDERS ═════════════════════════════════════════════
          TJ: drop GIFs in website/public/ and replace the placeholders:
            <img src="/todork/scan-demo.gif"  alt="Generic scan"   width="100%" />
            <img src="/todork/blame-demo.gif" alt="--blame demo"   width="100%" />
      ═══════════════════════════════════════════════════════════════════ */}
      <section class="section">
        <div class="container">
          <p class="section-label center">see it run</p>
          <h2 class="section-title center">In the wild</h2>
          <div class="gif-grid">
            <div class="gif-card">
              <img src="/todork/todork.gif" alt="todork scanning a codebase" class="gif-img" />
              <div class="gif-caption">Generic scan</div>
              <div class="gif-sub">todork ./your-project</div>
            </div>
            <div class="gif-card">
              <img src="/todork/todork_blame.gif" alt="todork with --blame flag" class="gif-img" />
              <div class="gif-caption">Git blame enrichment</div>
              <div class="gif-sub">todork ./your-project --blame</div>
            </div>
          </div>
        </div>
      </section>

      <hr class="divider" />

      {/* ══ FEATURES ═════════════════════════════════════════════════════ */}
      <section class="section">
        <div class="container">
          <p class="section-label center">why todork</p>
          <h2 class="section-title center">Built for speed &amp; signal</h2>
          <div class="features-grid">
            <div class="feat-card"><span class="feat-icon">⚡</span><div><div class="feat-name">Ripgrep-class speed</div><div class="feat-desc">Parallel walks with <code>ignore::WalkParallel</code> and multi-pattern AhoCorasick matching.</div></div></div>
            <div class="feat-card"><span class="feat-icon">🦀</span><div><div class="feat-name">Written in Rust</div><div class="feat-desc">Zero-overhead abstractions, memory-safe by default, no runtime required.</div></div></div>
            <div class="feat-card"><span class="feat-icon">🔍</span><div><div class="feat-name">8 annotation types</div><div class="feat-desc">TODO, FIXME, HACK, NOTE, BUG, OPTIMIZE, REVIEW, XXX - all configurable with <code>--tags</code>.</div></div></div>
            <div class="feat-card"><span class="feat-icon">📋</span><div><div class="feat-name">Multiple output formats</div><div class="feat-desc">Human-readable text, machine-parseable JSON, and GitHub Annotations for CI.</div></div></div>
            <div class="feat-card"><span class="feat-icon">🔀</span><div><div class="feat-name">Git blame enrichment</div><div class="feat-desc">Add <code>--blame</code> to see who wrote each annotation, when, and which commit.</div></div></div>
            <div class="feat-card"><span class="feat-icon">⚙️</span><div><div class="feat-name">Fully configurable</div><div class="feat-desc">Glob patterns, depth limits, filesize caps, thread count, gitignore awareness.</div></div></div>
          </div>
        </div>
      </section>

      <hr class="divider" />

      {/* ══ QUICK START ══════════════════════════════════════════════════ */}
      <section class="section">
        <div class="container">
          <p class="section-label center">get going</p>
          <h2 class="section-title center">Quick start</h2>
          <div class="qs-grid">
            <div class="qs-card"><div class="qs-num">01 - INSTALL</div><div class="qs-code"><span class="q-prompt">$ </span>curl -fsSL …/install.sh | sh</div><div class="qs-desc">One-liner. Detects OS &amp; arch, verifies SHA-256, drops the binary in <code>~/.local/bin</code>.</div></div>
            <div class="qs-card"><div class="qs-num">02 - SCAN</div><div class="qs-code"><span class="q-prompt">$ </span>todork ./src</div><div class="qs-desc">Scans recursively, respects <code>.gitignore</code>, prints every annotation found.</div></div>
            <div class="qs-card"><div class="qs-num">03 - BLAME</div><div class="qs-code"><span class="q-prompt">$ </span>todork . --blame</div><div class="qs-desc">Enriches every result with author, age and commit hash via <code>git blame</code>.</div></div>
            <div class="qs-card"><div class="qs-num">04 - CI</div><div class="qs-code"><span class="q-prompt">$ </span>todork . --format github-annotations</div><div class="qs-desc">Outputs native GitHub Annotations - results surface inline in PR diffs.</div></div>
          </div>
        </div>
      </section>

      <hr class="divider" />

      {/* ══ FOOTER ═══════════════════════════════════════════════════════ */}
      <footer class="footer">
        <div class="container">
          <p class="footer-text">
            <a href={GH} target="_blank" rel="noopener noreferrer">tjklint/todork</a>
            <span class="fsep">·</span>MIT Licensed
            <span class="fsep">·</span>Built with 🦀 Rust
            <span class="fsep">·</span>
            <a href={`${GH}/releases`} target="_blank" rel="noopener noreferrer">Releases</a>
            <span class="fsep">·</span>
            <a href={`${GH}/issues`} target="_blank" rel="noopener noreferrer">Issues</a>
          </p>
        </div>
      </footer>
    </>
  );
});

export const head: DocumentHead = {
  title: 'todork - hyper-fast TODO scanner',
  meta: [
    { name: 'description',        content: 'Scan your entire codebase for TODO, FIXME, HACK and more in milliseconds. Written in Rust. Open source. MIT licensed.' },
    { property: 'og:title',       content: 'todork' },
    { property: 'og:description', content: 'Hyper-fast annotation scanner for codebases. Written in Rust.' },
    { property: 'og:type',        content: 'website' },
    { name: 'theme-color',        content: '#07090f' },
    { name: 'twitter:card',       content: 'summary' },
  ],
  links: [
    { rel: 'preconnect', href: 'https://fonts.googleapis.com' },
    { rel: 'preconnect', href: 'https://fonts.gstatic.com', crossOrigin: 'anonymous' },
    {
      rel: 'stylesheet',
      href: 'https://fonts.googleapis.com/css2?family=Inter:opsz,wght@14..32,400;14..32,500;14..32,600;14..32,700;14..32,800&family=JetBrains+Mono:wght@400;500;700&display=swap',
    },
  ],
};
