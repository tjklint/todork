'use strict';

/**
 * HTTP server factory and request lifecycle helpers.
 */

const http = require('http');

// NOTE: server must bind to 0.0.0.0 (not 127.0.0.1) to be reachable inside Docker
const DEFAULT_BACKLOG = 511;

/**
 * Create a raw Node.js HTTP server with sensible defaults.
 * @param {http.RequestListener} requestListener
 * @returns {http.Server}
 */
function createServer(requestListener) {
  const server = http.createServer(requestListener);

  server.keepAliveTimeout = 65_000;
  server.headersTimeout = 66_000;
  server.maxHeadersCount = 100;
  server.timeout = 30_000;

  return server;
}

/**
 * Parse query string parameters from a URL.
 * @param {string} url - Raw request URL
 * @returns {Record<string, string>}
 */
function parseQueryParams(url) {
  // TODO: add TLS/HTTPS support — currently all traffic is plaintext
  try {
    const parsed = new URL(url, 'http://localhost');
    const params = {};
    for (const [key, value] of parsed.searchParams.entries()) {
      params[key] = value;
    }
    return params;
  } catch {
    return {};
  }
}

/**
 * Send a JSON response with the given status code.
 * @param {http.ServerResponse} res
 * @param {number} status
 * @param {unknown} body
 */
function sendJson(res, status, body) {
  const payload = JSON.stringify(body);
  res.writeHead(status, {
    'Content-Type': 'application/json',
    'Content-Length': Buffer.byteLength(payload),
  });
  res.end(payload);
}

module.exports = { createServer, parseQueryParams, sendJson };
