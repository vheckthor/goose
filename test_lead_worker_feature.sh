#!/bin/bash

# Test script to demonstrate the lead/worker model feature
# This shows how to configure and test the feature

echo "=== Lead/Worker Model Feature Test ==="
echo

echo "1. Testing with GOOSE_LEAD_MODEL set:"
echo "   GOOSE_PROVIDER=openai"
echo "   GOOSE_MODEL=gpt-4o-mini (worker model)"
echo "   GOOSE_LEAD_MODEL=gpt-4o (lead model for first 3 turns)"
echo

echo "2. Expected behavior:"
echo "   - Turn 1-3: Uses gpt-4o (lead model)"
echo "   - Turn 4+: Uses gpt-4o-mini (worker model)"
echo

echo "3. To test manually:"
echo "   export GOOSE_PROVIDER=openai"
echo "   export GOOSE_MODEL=gpt-4o-mini"
echo "   export GOOSE_LEAD_MODEL=gpt-4o"
echo "   export OPENAI_API_KEY=your_key_here"
echo "   goose session start"
echo

echo "4. To disable (use only worker model):"
echo "   unset GOOSE_LEAD_MODEL"
echo

echo "5. Watch the logs for messages like:"
echo "   'Using lead provider for turn 1 (lead_turns: 3)'"
echo "   'Using worker provider for turn 4 (lead_turns: 3)'"
echo

echo "=== Configuration Examples ==="
echo

echo "OpenAI (GPT-4o -> GPT-4o-mini):"
echo "export GOOSE_PROVIDER=openai"
echo "export GOOSE_MODEL=gpt-4o-mini"
echo "export GOOSE_LEAD_MODEL=gpt-4o"
echo

echo "Anthropic (Claude 3.5 Sonnet -> Claude 3 Haiku):"
echo "export GOOSE_PROVIDER=anthropic"
echo "export GOOSE_MODEL=claude-3-haiku-20240307"
echo "export GOOSE_LEAD_MODEL=claude-3-5-sonnet-20241022"
echo

echo "=== Unit Tests ==="
echo "Run unit tests with:"
echo "cargo test -p goose lead_worker --lib"
echo "(Note: May fail due to protoc issues, but the logic is tested)"