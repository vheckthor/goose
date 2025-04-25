---
sidebar_position: 20
title: Create a Session Sharing Service
sidebar_label: Create a Sharing Service
---


This document covers the implementation details for setting up a Goose session sharing server. The discussion focused on:
1. How to implement the required server endpoints for session sharing
2. The flow of sharing sessions between Goose Desktop clients
3. Implementation of a web interface for shared session links
4. Security considerations and best practices

### Set up a server to enable session sharing

The server uses a [Goose session sharing API](/docs/Session-Sharing/SSE-api-reference) to implement two main endpoints and handle proper authentication and security:

1. **API Requirements**
```
POST /sessions/share
GET /sessions/share/:shareToken
```

2. **API Interface**
```typescript
// POST /sessions/share Request Body
interface CreateSessionRequest {
  working_dir: string;
  messages: Message[];
  description: string;
  base_url: string;
  total_tokens: number | null;
}

// POST /sessions/share Response
interface CreateSessionResponse {
  share_token: string;
}

// GET /sessions/share/:shareToken Response
interface SharedSessionDetails {
  share_token: string;
  created_at: number;
  base_url: string;
  description: string;
  working_dir: string;
  messages: Message[];
  message_count: number;
  total_tokens: number | null;
}
```

3. **Security Requirements**
- The server must verify a secret key for authentication
- The server should be configured with HTTPS
- CORS must be properly configured to allow requests from Goose desktop clients
- Consider implementing rate limiting for shared session creation

4. **Example Server Implementation**
```javascript
const express = require('express');
const app = express();
const cors = require('cors');

app.use(express.json());
app.use(cors({
  origin: true,
  credentials: true
}));

// In-memory storage (replace with database in production)
const sessions = new Map();

// Middleware to verify secret key
const verifySecretKey = (req, res, next) => {
  const secretKey = req.headers['x-secret-key'];
  if (!secretKey || secretKey !== process.env.SECRET_KEY) {
    return res.status(401).json({ error: 'Unauthorized' });
  }
  next();
};

// Create shared session
app.post('/sessions/share', verifySecretKey, (req, res) => {
  const {
    working_dir,
    messages,
    description,
    base_url,
    total_tokens
  } = req.body;

  const shareToken = generateUniqueToken();
  const session = {
    share_token: shareToken,
    created_at: Date.now(),
    base_url,
    description,
    working_dir,
    messages,
    message_count: messages.length,
    total_tokens
  };

  sessions.set(shareToken, session);
  res.json({ share_token: shareToken });
});

// Get shared session
app.get('/sessions/share/:shareToken', verifySecretKey, (req, res) => {
  const session = sessions.get(req.params.shareToken);
  if (!session) {
    return res.status(404).json({ error: 'Session not found' });
  }
  res.json(session);
});

app.listen(3000, () => {
  console.log('Goose sharing server running on port 3000');
});
```

### Q: If the Goose Desktop calls my CreateSessionRequest endpoint and gets back an URL and a session token, how should that URL and token get to another Goose desktop client to allow sharing?

The process involves creating a web interface that handles shared session links:

1. **URL Format**
```
https://your-sharing-service.com/share/{share_token}
```

2. **Web Route Implementation**
```javascript
app.get('/share/:token', (req, res) => {
  const token = req.params.token;
  const baseUrl = `${req.protocol}://${req.get('host')}`;
  
  // Send HTML page
  res.send(`
    <!DOCTYPE html>
    <html>
    <head>
      <title>Goose Shared Session</title>
      <meta name="viewport" content="width=device-width, initial-scale=1">
      <style>
        body {
          font-family: system-ui, -apple-system, sans-serif;
          max-width: 800px;
          margin: 0 auto;
          padding: 20px;
          line-height: 1.6;
        }
        .button {
          display: inline-block;
          background: #4F46E5;
          color: white;
          padding: 12px 24px;
          border-radius: 6px;
          text-decoration: none;
          margin: 10px 0;
        }
        .info {
          background: #F3F4F6;
          padding: 16px;
          border-radius: 8px;
          margin: 20px 0;
        }
      </style>
    </head>
    <body>
      <h1>Goose Shared Session</h1>
      
      <div class="info">
        <p>Someone has shared a Goose session with you!</p>
      </div>

      <a href="goose://share/${token}?base_url=${encodeURIComponent(baseUrl)}" class="button">
        Open in Goose Desktop
      </a>

      <div class="info">
        <h3>Don't have Goose Desktop?</h3>
        <p>To view this shared session:</p>
        <ol>
          <li>Install Goose Desktop from <a href="https://github.com/block/goose/releases">GitHub Releases</a></li>
          <li>Configure session sharing in Settings:</li>
          <ul>
            <li>Enable session sharing</li>
            <li>Set the base URL to: <code>${baseUrl}</code></li>
          </ul>
          <li>Click the "Open in Goose Desktop" button above</li>
        </ol>
      </div>

      <script>
        // Attempt to open Goose Desktop automatically
        document.addEventListener('DOMContentLoaded', () => {
          window.location.href = 'goose://share/${token}?base_url=${encodeURIComponent(baseUrl)}';
          
          // After a delay, if we're still here, the app didn't open
          setTimeout(() => {
            document.getElementById('manual-instructions').style.display = 'block';
          }, 2000);
        });
      </script>
    </body>
    </html>
  `);
});
```

3. **Custom Protocol Handler in Goose Desktop**
```typescript
// In your Electron main process (main.ts)
if (process.defaultApp) {
  if (process.argv.length >= 2) {
    app.setAsDefaultProtocolClient('goose', process.execPath, [
      path.resolve(process.argv[1])
    ]);
  }
} else {
  app.setAsDefaultProtocolClient('goose');
}

// Handle the protocol
app.on('open-url', (event, url) => {
  event.preventDefault();
  
  const urlObj = new URL(url);
  if (urlObj.protocol === 'goose:') {
    // Extract token and base_url from the URL
    const token = urlObj.pathname.replace('/share/', '');
    const baseUrl = urlObj.searchParams.get('base_url');
    
    // Handle the shared session
    handleSharedSession(token, baseUrl);
  }
});
```

4. **Flow**
```
User A                     Server                      User B
  |                          |                           |
  |-- Create share --------->|                           |
  |<-- Token & URL ----------|                           |
  |                          |                           |
  |-- Share URL with User B->|                           |
  |                          |                           |
  |                          |<-- Open URL in browser ---|
  |                          |--- Serve HTML page ------>|
  |                          |                           |
  |                          |   [Click "Open in Goose"] |
  |                          |                           |
  |                          |<-- Request session -------|
  |                          |--- Return session ------->|
```

## Additional Considerations

1. **Security**
- Use unguessable share tokens (UUIDs recommended)
- Implement token expiration
- Validate base URL matches between clients
- Add rate limiting
- Use HTTPS
- Add appropriate security headers

2. **Error Handling**
- Invalid or expired tokens
- Base URL mismatches
- Sharing not enabled
- Network errors

3. **Client Configuration**
- Both users need session sharing enabled
- Both users must use the same base URL
- Base URL can be set via:
  - Environment variable (`GOOSE_BASE_URL_SHARE`)
  - User configuration in settings