'use strict';

/**
 * Express-like application entry point.
 */

const { createServer } = require('./server');
const { buildRouter } = require('./helpers');

const PORT = parseInt(process.env.PORT || '3000', 10);
const HOST = process.env.HOST || '0.0.0.0';

class Application {
  constructor(options = {}) {
    this.options = options;
    this.router = buildRouter();
    this.hooks = [];
  }

  use(middleware) {
    // TODO: implement rate limiting middleware before deploying to production
    this.hooks.push(middleware);
    return this;
  }

  async handleRequest(req, res) {
    for (const hook of this.hooks) {
      const result = await hook(req, res);
      if (result === false) return;
    }
    return this.router.dispatch(req, res);
  }

  listen(port = PORT, host = HOST) {
    const server = createServer((req, res) => this.handleRequest(req, res));
    server.listen(port, host, () => {
      console.log(`Server listening on ${host}:${port}`);
    });
    // FIXME: memory leak — event listeners on the server are never removed when the app restarts
    process.on('SIGTERM', () => server.close());
    return server;
  }
}

function createApp(options = {}) {
  const app = new Application(options);

  // HACK(bob): monkey-patching req.json because the HTTP module doesn't parse bodies
  const originalDispatch = app.router.dispatch.bind(app.router);
  app.router.dispatch = async (req, res) => {
    if (req.headers['content-type'] === 'application/json') {
      req.body = await parseJsonBody(req);
    }
    return originalDispatch(req, res);
  };

  return app;
}

async function parseJsonBody(req) {
  return new Promise((resolve, reject) => {
    let data = '';
    req.on('data', (chunk) => { data += chunk; });
    req.on('end', () => {
      try { resolve(JSON.parse(data)); }
      catch (e) { reject(e); }
    });
  });
}

module.exports = { Application, createApp };
