import express from 'express';
import { createProxyMiddleware } from 'http-proxy-middleware';
import type { Plugin } from 'http-proxy-middleware';
import cors from 'cors';
import yaml from 'js-yaml';
import fs from 'fs';
import NodeCache from 'node-cache';
import { IncomingMessage, ServerResponse } from 'http';

const app = express();
const port = 3001;

// const REGISTRY_API_BASE = 'https://demo.registry.azure-mcp.net';
const REGISTRY_API_BASE = 'http://localhost:8080';

// Initialize cache with 5 minute TTL
const cache = new NodeCache({ stdTTL: 1 });
const SERVERS_CACHE_KEY = 'servers_list';

interface AllowedConfig {
  extensions: Array<{ name: string, command: string }>;
}

// Allowlisted extensions
const allowedConfig = yaml.load(fs.readFileSync('./allowed.yaml', 'utf8')) as AllowedConfig;
const allowedExtensions = new Set(
  allowedConfig.extensions.map((ext) => ext.name)
);

console.log(allowedExtensions);

app.use(cors());

const serverFilterPlugin: Plugin = (proxyServer) => {
  proxyServer.on('proxyRes', (
    proxyRes: IncomingMessage,
    req: IncomingMessage,
    res: ServerResponse
  ) => {
    // Only process /api/v0/servers endpoint
    if (req.url !== '/v0/servers') {
      proxyRes.pipe(res);
      return;
    }

    res.setHeader('content-type', 'application/json');

    // Check cache first
    const cachedServers = cache.get(SERVERS_CACHE_KEY);
    if (cachedServers) {
      console.log('Serving servers list from cache');
      res.end(JSON.stringify({ servers: cachedServers }));
      return;
    }

    let body = '';
    proxyRes.on('data', (chunk: Buffer) => {
      body += chunk;
    });

    proxyRes.on('end', () => {
      try {
        const parsedBody = JSON.parse(body);
        const responseData = parsedBody.servers;

        // If the response has an array of servers, filter based on allowed extensions
        if (Array.isArray(responseData)) {
          const filteredServers = responseData.filter(server => {
            console.log(server.name);
            console.log(allowedExtensions.has(server.name));
            return allowedExtensions.has(server.name)
          });
          // Cache filtered servers and respond with:
          //   - original data
          //   - servers replaced with filtered servers
          cache.set(SERVERS_CACHE_KEY, filteredServers);
          res.end(JSON.stringify({ ...parsedBody, servers: filteredServers }));
        } else {
          // Return original response
          res.end(body);
        }
      } catch (error) {
        // Return original response
        console.error('Error handling response:', error);
        res.end(body);
      }
    });
  });
};

app.use('/api', createProxyMiddleware({
  target: REGISTRY_API_BASE,
  changeOrigin: true,
  selfHandleResponse: true,
  plugins: [serverFilterPlugin],
}));

app.listen(port, () => console.log(`registry proxy running at http://localhost:${port}`));