# Lead/Worker Model Feature with Smart Failure Detection

This feature allows Goose to use a more capable "lead" model for the first 3 turns of a conversation, then automatically switch to the regular configured "worker" model for subsequent turns. Additionally, it includes **intelligent failure detection** that can identify both technical failures and task-level failures, automatically falling back to the lead model when needed.

## Configuration Options

### Option 1: Environment Variables (Simple)
```bash
export GOOSE_PROVIDER="openai"
export GOOSE_MODEL="gpt-4o-mini"           # Worker model
export GOOSE_LEAD_MODEL="gpt-4o"          # Lead model
```

### Option 2: YAML Configuration (Simple)
Create or edit `~/.config/goose/config.yaml`:

```yaml
# Standard configuration
GOOSE_PROVIDER: openai
GOOSE_MODEL: gpt-4o-mini
GOOSE_LEAD_MODEL: gpt-4o
```

### Option 3: YAML Configuration (Advanced)
Create or edit `~/.config/goose/config.yaml`:

```yaml
# Standard configuration
GOOSE_PROVIDER: openai
GOOSE_MODEL: gpt-4o-mini

# Lead/Worker configuration
lead_worker:
  enabled: true
  lead_model: gpt-4o
  lead_turns: 3
  failure_threshold: 2
  fallback_turns: 2
```

### Option 4: Cross-Provider Configuration (Most Powerful)
```yaml
GOOSE_PROVIDER: openai
GOOSE_MODEL: gpt-4o-mini

lead_worker:
  enabled: true
  lead_provider: openai
  lead_model: gpt-4o
  worker_provider: anthropic
  worker_model: claude-3-haiku-20240307
  lead_turns: 3
  failure_threshold: 2
  fallback_turns: 2
```

## Configuration Precedence

The system respects the following precedence order:
1. **Environment variables** (highest) - `GOOSE_LEAD_MODEL` overrides everything
2. **YAML `lead_worker` section** - Advanced configuration with cross-provider support
3. **YAML flat keys** - `GOOSE_LEAD_MODEL` in config file
4. **Regular provider** (lowest) - Standard single-model operation

This ensures full backward compatibility while enabling advanced features.

## YAML Configuration Reference

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `enabled` | boolean | false | Enable lead/worker mode |
| `lead_provider` | string | main provider | Provider for lead model |
| `lead_model` | string | required | Model name for lead |
| `worker_provider` | string | main provider | Provider for worker model |
| `worker_model` | string | main model | Model name for worker |
| `lead_turns` | number | 3 | Initial turns using lead model |
| `failure_threshold` | number | 2 | Failures before fallback |
| `fallback_turns` | number | 2 | Turns in fallback mode |

## How it works

### Normal Operation:
1. **Turns 1-3**: Uses the model specified in `GOOSE_LEAD_MODEL`
2. **Turn 4+**: Uses the model specified in `GOOSE_MODEL`
3. **New session**: Turn counter resets, starts with lead model again

### Smart Failure Detection:
The system detects two types of failures:

#### 1. **Technical Failures** (API/Network issues):
- Network timeouts, API errors
- Authentication failures
- Rate limiting, context length exceeded

#### 2. **Task-Level Failures** (Model performance issues):
- **Tool execution failures**: Commands that return errors, file operations that fail
- **Error patterns in output**: Detects "error:", "failed:", "exception:", "traceback", etc.
- **User correction patterns**: Phrases like "that's wrong", "try again", "that doesn't work"
- **Test/compilation failures**: "test failed", "compilation failed", "assertion failed"

### Automatic Fallback:
1. **Failure Tracking**: Counts consecutive failures of either type
2. **Fallback Trigger**: After 2 consecutive failures, switches back to lead model
3. **Fallback Duration**: Uses lead model for 2 turns to help get back on track
4. **Recovery**: Returns to worker model after successful fallback period

## Examples

### Scenario 1: Tool Execution Failures
```
Turn 4: GPT-4o-mini tries to edit file → "Permission denied" error
Turn 5: GPT-4o-mini tries different approach → "File not found" error
Turn 6: System detects 2 failures → Switches to GPT-4o (fallback mode)
Turn 7: GPT-4o successfully fixes the issue → Fallback continues
Turn 8: GPT-4o completes task → Exits fallback, returns to GPT-4o-mini
```

### Scenario 2: User Corrections
```
Turn 4: GPT-4o-mini suggests solution A
User: "That's wrong, try a different approach"
Turn 5: GPT-4o-mini suggests solution B  
User: "That doesn't work either, let me correct you..."
Turn 6: System detects user correction patterns → Switches to GPT-4o
```

### Scenario 3: Code/Test Failures
```
Turn 4: GPT-4o-mini writes code → Tool runs test → "Test failed: AssertionError"
Turn 5: GPT-4o-mini fixes code → Tool runs test → "Compilation failed: syntax error"
Turn 6: System detects error patterns → Switches to GPT-4o for better debugging
```

## Configuration Examples

### OpenAI: Use GPT-4o for planning, GPT-4o-mini for execution
```bash
export GOOSE_PROVIDER="openai"
export GOOSE_MODEL="gpt-4o-mini"
export GOOSE_LEAD_MODEL="gpt-4o"
```

### Anthropic: Use Claude 3.5 Sonnet for initial reasoning, Claude 3 Haiku for follow-up
```bash
export GOOSE_PROVIDER="anthropic"  
export GOOSE_MODEL="claude-3-haiku-20240307"
export GOOSE_LEAD_MODEL="claude-3-5-sonnet-20241022"
```

### YAML Configuration: Cross-provider setup
```yaml
GOOSE_PROVIDER: openai
GOOSE_MODEL: gpt-4o-mini

lead_worker:
  enabled: true
  lead_provider: anthropic
  lead_model: claude-3-5-sonnet-20241022
  worker_provider: openai
  worker_model: gpt-4o-mini
```

### Disable (default behavior)
```bash
unset GOOSE_LEAD_MODEL
# Only GOOSE_MODEL will be used for all turns
```

## Log Messages

Watch for these log messages to understand the behavior:

### Normal Operation:
- `"Using lead (initial) provider for turn 1 (lead_turns: 3)"`
- `"Using worker provider for turn 4 (lead_turns: 3)"`

### Failure Detection:
- `"Task failure detected in response (failure count: 1)"`
- `"Technical failure detected (failure count: 2)"`
- `"Tool execution failure detected: Permission denied"`
- `"User correction pattern detected in text"`

### Fallback Mode:
- `"🔄 SWITCHING TO LEAD MODEL: Entering fallback mode after 2 consecutive task failures - using lead model for 2 turns"`
- `"🔄 Using lead (fallback) provider for turn 7 (FALLBACK MODE: 1 turns remaining)"`
- `"✅ SWITCHING BACK TO WORKER MODEL: Exiting fallback mode - worker model resumed"`

## Detected Failure Patterns

### Tool Output Errors:
- `error:`, `failed:`, `exception:`, `traceback`
- `syntax error`, `permission denied`, `file not found`
- `command not found`, `compilation failed`
- `test failed`, `assertion failed`

### User Correction Phrases:
- `"that's wrong"`, `"that's not right"`, `"that doesn't work"`
- `"try again"`, `"let me correct"`, `"actually, "`
- `"no, that's"`, `"that's incorrect"`, `"fix this"`
- `"this is broken"`, `"this doesn't"`
- Starting with: `"no,"`, `"wrong"`, `"incorrect"`

## Benefits

- **Cost optimization**: Use expensive models only when needed
- **Performance**: Get high-quality initial responses, then faster follow-ups
- **Reliability**: Automatically recover from both technical and task failures
- **Intelligence**: Detects when the model is struggling with the actual task, not just API issues
- **Self-healing**: No manual intervention needed when worker model gets stuck
- **User-aware**: Recognizes when users are expressing dissatisfaction and correcting the model

## Implementation Details

### Core Components

The lead/worker feature is implemented across several key components:

#### Provider Layer (`crates/goose/src/providers/`)
- **`LeadWorkerProvider`**: Main wrapper that manages switching between lead and worker providers
- **`LeadWorkerConfig`**: Configuration structure for YAML-based setup
- **Factory pattern**: Handles precedence and provider creation logic

#### CLI Integration (`crates/goose-cli/src/session/`)
- **Startup logging**: Displays model information when sessions begin
- **Session management**: Integrates with existing session workflow

### Key Features Implemented

#### ✅ Startup Logging
The system provides clear visibility into which models are configured:

**Tracing Integration:**
```rust
tracing::info!(
    "🤖 Lead/Worker Mode Enabled: Lead model (first 3 turns): {}, Worker model (turn 4+): {}, Auto-fallback on failures: Enabled",
    lead_model,
    worker_model
);
```

**Session Header Display:**
```
starting session | provider: openai lead model: gpt-4o worker model: gpt-4o-mini
```

#### ✅ Turn-by-Turn Logging
- `"Using lead (initial) provider for turn 1 (lead_turns: 3)"`
- `"Using worker provider for turn 4 (lead_turns: 3)"`
- `"🔄 Using lead (fallback) provider for turn 7 (FALLBACK MODE: 1 turns remaining)"`

#### ✅ Configuration Support
- **Environment variables**: Simple setup with `GOOSE_LEAD_MODEL`
- **YAML flat keys**: `GOOSE_LEAD_MODEL` in config file
- **YAML `lead_worker` section**: Advanced setup with cross-provider support
- **Proper precedence handling**: Environment > YAML lead_worker > YAML flat > defaults

#### ✅ Comprehensive Testing
- Unit tests for configuration parsing and defaults
- Integration tests for provider switching logic
- Fallback behavior verification
- All tests pass successfully

### Configuration Precedence Implementation

The system implements a clear precedence order in `factory.rs`:

1. **Environment variables** (highest): `GOOSE_LEAD_MODEL` env var
2. **YAML `lead_worker` section**: Full configuration control
3. **YAML flat keys**: `GOOSE_LEAD_MODEL` in config file  
4. **Regular provider** (lowest): Standard single-model operation

### Files Modified/Created

**Core Implementation:**
- `crates/goose/src/providers/base.rs` - Added `LeadWorkerProviderTrait`
- `crates/goose/src/providers/lead_worker.rs` - Main provider implementation
- `crates/goose/src/providers/factory.rs` - Configuration and creation logic
- `crates/goose-cli/src/session/builder.rs` - Startup logging integration

**Documentation:**
- `crates/goose/docs/lead-worker/LEAD_WORKER_FEATURE.md` - This comprehensive guide
- `crates/goose/docs/lead-worker/README.md` - Quick start guide
- `crates/goose/docs/lead-worker/example-config.yaml` - Example configuration

The implementation is complete, tested, and provides full backward compatibility while enabling advanced lead/worker functionality.