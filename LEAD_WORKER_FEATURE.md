# Lead/Worker Model Feature

This feature allows Goose to use a more capable "lead" model for the first 3 turns of a conversation, then automatically switch to the regular configured "worker" model for subsequent turns.

## Usage

Simply set the `GOOSE_LEAD_MODEL` environment variable to enable this feature:

```bash
export GOOSE_PROVIDER="openai"
export GOOSE_MODEL="gpt-4o-mini"           # This becomes the worker model
export GOOSE_LEAD_MODEL="gpt-4o"          # This is used for first 3 turns
```

## How it works

1. **Turns 1-3**: Uses the model specified in `GOOSE_LEAD_MODEL`
2. **Turn 4+**: Uses the model specified in `GOOSE_MODEL`
3. **New session**: Turn counter resets, starts with lead model again

## Examples

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

### Disable (default behavior)
```bash
unset GOOSE_LEAD_MODEL
# Only GOOSE_MODEL will be used for all turns
```

## Benefits

- **Cost optimization**: Use expensive models only when needed
- **Performance**: Get high-quality initial responses, then faster follow-ups
- **Workflow optimization**: Better planning/reasoning upfront, efficient execution after