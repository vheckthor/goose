---
title: SSE API Reference
---


FastAPI service version 0.1.0

## Endpoints

### MCP Endpoints

#### GET /messages

Messages endpoint for SSE communication

**Description**: This endpoint is used for posting messages to SSE clients.
Note: This route is for documentation purposes only.
The actual implementation is handled by the SSE transport.

**Responses**:
| Status | Description | Content Type |
|--------|-------------|--------------|
| 200 | Successful Response | application/json |

#### GET /sse

SSE endpoint that connects to the MCP server

**Description**: This endpoint establishes a Server-Sent Events connection with the client
and forwards communication to the Model Context Protocol server.

**Responses**:
| Status | Description | Content Type |
|--------|-------------|--------------|
| 200 | Successful Response | application/json |

### Status Endpoints

#### GET /_status

Return the status of the service.

**Description**: This endpoint is REQUIRED for the service to be considered healthy.

**Responses**:
| Status | Description | Content Type | Schema |
|--------|-------------|--------------|--------|
| 200 | Successful Response | application/json | StatusResponse |

#### GET /api/ping/

A simple hello-world endpoint.

**Responses**:
| Status | Description | Content Type | Schema |
|--------|-------------|--------------|--------|
| 200 | Successful Response | application/json | PingResponse |

### Session Management

#### POST /api/sessions/share

Share a session by creating a unique token and storing the session data.

**Request Body**: Required
- Schema: SessionShareRequest
- Content Type: application/json

**Responses**:
| Status | Description | Content Type | Schema |
|--------|-------------|--------------|--------|
| 200 | Successful Response | application/json | SessionShareResponse |
| 422 | Validation Error | application/json | HTTPValidationError |

#### GET /api/sessions/share/share_token

Retrieve an existing shared session by token.

**Parameters**:
| Name | In | Required | Type | Description |
|------|------|----------|------|-------------|
| share_token | path | Yes | string | Share Token |

**Responses**:
| Status | Description | Content Type | Schema |
|--------|-------------|--------------|--------|
| 200 | Successful Response | application/json | SessionWithMessages |
| 422 | Validation Error | application/json | HTTPValidationError |

#### GET /api/sessions/

Returns a list of shareTokens and minimal metadata for all shared sessions in DynamoDB.

**Responses**:
| Status | Description | Content Type | Schema |
|--------|-------------|--------------|--------|
| 200 | Successful Response | application/json | Array[Session] |

## Data Models

### BaseMessage

A base message model.

**Properties**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| created | integer | Yes | Creation timestamp |
| role | string | Yes | Message role |
| content | array[object] | Yes | Message content |

### Session

Data model for shared sessions in DynamoDB.

**Properties**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| share_token | string | Yes | Unique share token |
| created | integer | Yes | Creation timestamp |
| base_url | string | Yes | Base URL |
| working_dir | string | Yes | Working directory |
| description | string | Yes | Session description |
| message_count | integer | Yes | Number of messages |
| total_tokens | integer/null | No | Total token count |

### SessionShareRequest

Request model for sharing a session.

**Properties**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| messages | array[BaseMessage] | Yes | Array of messages |
| working_dir | string | Yes | Working directory |
| description | string | Yes | Session description |
| base_url | string | Yes | Base URL |
| total_tokens | integer/null | No | Total token count |

### SessionShareResponse

Response model for sharing a session.

**Properties**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| share_token | string | Yes | Unique share token |

### SessionWithMessages

Extended Session model that includes messages - used for API responses.

**Properties**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| share_token | string | Yes | Unique share token |
| created | integer | Yes | Creation timestamp |
| base_url | string | Yes | Base URL |
| working_dir | string | Yes | Working directory |
| description | string | Yes | Session description |
| message_count | integer | Yes | Number of messages |
| total_tokens | integer/null | No | Total token count |
| messages | array[BaseMessage] | Yes | Array of messages |

### StatusResponse

Status response model for the _status endpoint.

**Properties**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| ok | boolean | Yes | Status flag |
| status | string | Yes | Status message |

### PingResponse

Ping response model.

**Properties**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| message | string | Yes | Response message |

### HTTPValidationError

**Properties**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| detail | array[ValidationError] | No | Validation error details |

### ValidationError

**Properties**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| loc | array[string/integer] | Yes | Error location |
| msg | string | Yes | Error message |
| type | string | Yes | Error type |