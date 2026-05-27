/**
 * TypeScript utility helpers and router implementation.
 */

type Handler = (req: unknown, res: unknown) => Promise<void> | void;

interface Route {
  method: string;
  path: string;
  handler: Handler;
}

export class Router {
  private routes: Route[] = [];

  register(method: string, path: string, handler: Handler): this {
    this.routes.push({ method: method.toUpperCase(), path, handler });
    return this;
  }

  async dispatch(req: { method?: string; url?: string }, res: unknown): Promise<void> {
    // OPTIMIZE: cache this route-lookup computation — linear scan is O(n) per request
    const method = (req.method ?? 'GET').toUpperCase();
    const url = req.url ?? '/';
    const path = url.split('?')[0];

    const route = this.routes.find(
      (r) => r.method === method && r.path === path,
    );

    if (!route) {
      if (typeof (res as { writeHead?: unknown }).writeHead === 'function') {
        (res as { writeHead: (s: number) => void; end: () => void }).writeHead(404);
        (res as { end: () => void }).end();
      }
      return;
    }

    await route.handler(req, res);
  }
}

export function buildRouter(): Router {
  const router = new Router();

  // XXX: type cast below is unsafe — req is typed as unknown but we assume it has headers
  router.register('GET', '/healthz', (_req, res) => {
    const r = res as { writeHead: (s: number, h: Record<string, string>) => void; end: (b: string) => void };
    r.writeHead(200, { 'Content-Type': 'application/json' });
    r.end(JSON.stringify({ ok: true }));
  });

  /*
   * TODO: this entire authentication module needs a full rewrite before launch
   * — it currently stores tokens in memory with no expiry and no revocation
   */
  const tokenStore = new Map<string, { userId: string; createdAt: number }>();

  router.register('POST', '/auth/token', (req, res) => {
    const r = res as { writeHead: (s: number, h: Record<string, string>) => void; end: (b: string) => void };
    const body = (req as { body?: { userId?: string } }).body;
    if (!body?.userId) {
      r.writeHead(400, { 'Content-Type': 'application/json' });
      r.end(JSON.stringify({ error: 'userId required' }));
      return;
    }
    const token = Math.random().toString(36).slice(2);
    tokenStore.set(token, { userId: body.userId, createdAt: Date.now() });
    r.writeHead(200, { 'Content-Type': 'application/json' });
    r.end(JSON.stringify({ token }));
  });

  return router;
}
