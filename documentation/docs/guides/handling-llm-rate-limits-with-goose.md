---
title: LLM Rate Limits
sidebar_position: 4
---

# Handling LLM Rate Limits

Rate limiting is the process of restricting the number of requests a user or application can send to an LLM API within a specific timeframe. LLM providers enforce this with the purpose of managing resources and preventing abuse. 

Since Goose is working very quickly to implement your tasks, you may need to manage rate limits imposed by the provider. If you frequently hit rate limits, consider upgrading your LLM plan to access higher tier limits or using OpenRouter.


## Using OpenRouter

OpenRouter provides a unified interface for LLMs that allows you to select and switch between different providers automatically - all under a single billing plan. With OpenRouter, you can utilize free models or purchase credits for paid models.

1. Go to [openrouter.ai](https://openrouter.ai) and create an account. 
2. Once verified, create your [API key](https://openrouter.ai/settings/keys).
3. Add your API key and OpenRouter configuration to your environment variables:

```bash
export GOOSE_PROVIDER="openrouter"
export OPENROUTER_API_KEY="..."
```

Now Goose will send your requests through OpenRouter which will automatically switch models when necessary to avoid interruptions due to rate limiting.

