---
sidebar_position: 2
---
# CLI Commands

Goose provides a command-line interface (CLI) with several commands for managing sessions, configurations and extensions. Below is a list of the available commands and their  descriptions:

## Commands

### help

Used to display the help menu

**Usage:**
```bash
goose --help
```

### version

Used to check the current Goose version you have installed

**Usage:**
```bash
goose --version
```

### agents

Used to list all available agents

**Usage:**
```bash
goose agents
```

### mcp

Run an enabled MCP server specified by `<name>` (e.g. 'Google Drive')

**Usage:**
```bash
goose mcp <name>
```

### session [options]

Start or resume sessions.

**Options:**
- **`-n, --name <NAME>`** : Name for the chat session (e.g., `'project-x'`)

- **`-p, --provider <PROVIDER>`**: Provider to use (e.g., `'openai'`, `'anthropic'`)

- **`-m, --model <MODEL>`**:Model to use (e.g., `'gpt-4'`, `'claude-3'`)

- **`-a, --agent <AGENT>`**: Agent version to use (e.g., `'default'`, `'v1'`), defaults to `'default'`

- **`-r, --resume`**: Resume a previous session (last used or specified by `--session`)


**Usage:**
```bash
goose session --resume
```

### run [options]

Execute commands from an instruction file or stdin

- **`-i, --instructions <FILE>`**: Path to instruction file containing commands  
- **`-t, --text <TEXT>`**: Input text to provide to Goose directly  
- **`-p, --provider <PROVIDER>`**: Provider to use (e.g., 'openai', 'anthropic')  
- **`-m, --model <MODEL>`**: Model to use (e.g., 'gpt-4', 'claude-3')  
- **`-n, --name <NAME>`**: Name for this run session (e.g., 'daily-tasks')  
- **`-a, --agent <AGENT>`**: Agent version to use (e.g., 'default', 'v1')  
- **`-r, --resume`**: Resume from a previous run  

**Usage:**
```bash
goose run --instructions plan.md
```

### configure [options]

Configure Goose to set providers, models, etc. 

- **`-p, --provider <PROVIDER>`**: AI Provider to use (e.g., 'openai', 'databricks', 'ollama')
- **`-m, --model <MODEL>`**: Model to use (e.g., 'gpt-4', 'llama2')


**Usage:**
```bash
goose configure --provider 'openai' --model 'gpt-4'
```

This command can also be run without any arguments, in which case you'll be prompted to make selections.

**Usage:**
```bash
goose configure
```