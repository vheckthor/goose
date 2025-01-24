---
sidebar_position: 2
title: Managing Goose Extensions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Extensions are add-ons that provide a way to extend the functionality of Goose. They also provide a way to connect Goose with applications and tools you already use in your workflow. These extensions can be used to add new features, automate tasks, or integrate with other systems. 

Extensions are based on the [Model Context Protocol (MCP)](https://github.com/modelcontextprotocol), so you can connect
Goose to a wide ecosystem of capabilities.

## Managing Extensions in Goose

You can add extensions to Goose through its configuration system. 

```
goose configure
```

After the first time setup, configure will let you choose to add extensions. Choose `Add Extension` to see your
options. You can also edit the config file directly, which is stored in `~/.config/goose/config.yaml`. 

### Built-in Extensions
Out of the box, Goose is installed with its `Developer` extension enabled, giving it a shell to run commands and a way to edit files
on your filesystem.

Goose has other built-in extensions that you can enable. To see your options:

1. Run `goose configure`
2. Choose `Add Extension`
3. Choose `Built-in Extension`

Alternatively, you can enable a built-in extension by specifying its name in this command:

```
goose mcp {name}
```

:::tip
All of Goose's built-in extensions are MCP servers in their own right. If you'd like
to use the MCP servers included with Goose with any other agent, you are free to do so.
:::

### MCP Servers

You can run any MCP server as a Goose extension. 

1. Run `goose configure`
2. Choose `Add Extension`
3. Choose `Command-line Extension`

You'll then be prompted to enter a command and any environment variables needed. For example, to connect to the [Fetch Server](https://github.com/modelcontextprotocol/servers/tree/main/src/fetch), enter `uvx mcp-server-fetch` as the command.

You can also edit the resulting config entry directly, which would look like this:

```yaml
extensions:
  fetch:
    name: fetch
    cmd: uvx
    args: [mcp-server-fetch]
    enabled: true
    envs: {}
    type: stdio
```


### Discovering Extensions

Goose comes with a [central directory](https://silver-disco-nvm6v4e.pages.github.io/) of extensions that you can install and use. You can install extensions from the Goose CLI or from the Goose GUI. The page will give you a test command to try out extensions, and if you want to keep them, you can add through `goose configure`. 

You can test out an extension for a single session with

```sh
goose session --with-extension "command to run"
```


## Starting a Session with Extensions

You can start a tailored goose session with specific extensions directly from the CLI. To do this, run the following command:

```bash
goose session --with-extension "{extension command}"
```

:::note
You may need to set necessary environment variables for the extension to work correctly.
```bash
goose session --with-extension "VAR=value command arg1 arg2"
```
:::

## Developing Extensions
Goose extensions are implemented with MCP - a system that allows AI models and agents to securely connect with local or remote resources using standard protocols. Learn how to build your own [extension as an MCP server](https://modelcontextprotocol.io/quickstart/server).
