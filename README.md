<div align="center">

# codename goose

_a local, extensible, open source AI agent that automates engineering tasks_

<p align="center">
  <a href="https://opensource.org/licenses/Apache-2.0">
    <img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg">
  </a>
  <a href="https://discord.gg/7GaTvbDwga">
    <img src="https://img.shields.io/discord/1287729918100246654?logo=discord&logoColor=white&label=Join+Us&color=blueviolet" alt="Discord">
  </a>
  <a href="https://github.com/block/goose/actions/workflows/ci.yml">
     <img src="https://img.shields.io/github/actions/workflow/status/block/goose/ci.yml?branch=main" alt="CI">
  </a>
</p>
</div>

goose is your on-machine AI agent, capable of automating complex development tasks from start to finish. More than just code suggestions, goose can build entire projects from scratch, write and execute code, debug failures, orchestrate workflows, and interact with external APIs - _autonomously_.

Whether you're prototyping an idea, refining existing code, or managing intricate engineering pipelines, goose adapts to your workflow and executes tasks with precision.

Designed for maximum flexibility, goose works with any LLM, seamlessly integrates with MCP servers, and is available as both a desktop app as well as CLI - making it the ultimate AI assistant for developers who want to move faster and focus on innovation. 

## Multiple Model Configuration

goose supports using different models for different purposes to optimize performance and cost:

### Lead/Worker Model Pattern
Use a powerful model for initial planning and complex reasoning, then switch to a faster/cheaper model for execution:

```bash
# Required: Enable lead model mode
export GOOSE_LEAD_MODEL=gpt-4o

# Optional: Configure behavior (defaults shown)
export GOOSE_LEAD_PROVIDER=anthropic  # Defaults to main provider
export GOOSE_LEAD_TURNS=3             # Initial lead model turns
export GOOSE_LEAD_FAILURE_THRESHOLD=2 # Failures before fallback
export GOOSE_LEAD_FALLBACK_TURNS=2    # Fallback lead model turns
```

### Planning Model Configuration  
Use a specialized model for the `/plan` command in CLI mode:

```bash
# Optional: Use different model for planning
export GOOSE_PLANNER_PROVIDER=openai
export GOOSE_PLANNER_MODEL=gpt-4
```

Both patterns help you balance model capabilities with cost and speed for optimal results. 


# Quick Links
- [Quickstart](https://block.github.io/goose/docs/quickstart)
- [Installation](https://block.github.io/goose/docs/getting-started/installation)
- [Tutorials](https://block.github.io/goose/docs/category/tutorials)
- [Documentation](https://block.github.io/goose/docs/category/getting-started)


# Goose Around with Us
- [Discord](https://discord.gg/block-opensource)
- [YouTube](https://www.youtube.com/@blockopensource)
- [LinkedIn](https://www.linkedin.com/company/block-opensource)
- [Twitter/X](https://x.com/blockopensource)
- [Bluesky](https://bsky.app/profile/opensource.block.xyz)
- [Nostr](https://njump.me/opensource@block.xyz)
