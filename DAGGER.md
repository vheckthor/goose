# Dagger Usage Guide

This document provides examples and explanations for using Dagger.

## Basic Concepts

Dagger operates by running commands in containers and allowing you to:
- Mount or copy files between your host and containers
- Chain operations together in a pipeline
- Execute commands within containers
- Export results back to your host

## Container Operations

### Starting a Container

```bash
dagger core container from --address="alpine:latest"
```

This creates a container from the Alpine Linux image.

### Working with Files

#### Mounting vs. Copying

Dagger provides two primary ways to work with files:

1. **Mounting** (`with-mounted-directory`): Changes to mounted directories reflect back to the host
2. **Copying** (`with-directory`): Changes only affect the container's copy (copy-on-write)

#### Copy-on-Write Example

This example copies files into the container, allowing modifications without affecting host files:

```bash
dagger core container from --address="alpine:latest" \
  with-directory --path="/workspace" --directory="." \
  with-workdir --path="/workspace" \
  with-exec --args="ls","-lah" \
  stdout
```

To demonstrate the copy-on-write behavior:

```bash
# This command modifies a file in the container but doesn't affect the host
dagger core container from --address="alpine:latest" \
  with-directory --path="/workspace" --directory="." \
  with-workdir --path="/workspace" \
  with-exec --args="sh","-c","echo '<!-- Modified in container -->' >> index.html && echo 'File modified in container'" \
  stdout
```

#### Mounting Example

This example mounts files, where changes in the container will affect host files:

```bash
dagger core container from --address="alpine:latest" \
  with-mounted-directory --path="/workspace" --directory="." \
  with-workdir --path="/workspace" \
  with-exec --args="ls","-lah" \
  stdout
```

To demonstrate how mounted files reflect changes back to host:

```bash
# Warning: This will modify your host files!
dagger core container from --address="alpine:latest" \
  with-mounted-directory --path="/workspace" --directory="." \
  with-workdir --path="/workspace" \
  with-exec --args="sh","-c","echo '<!-- Modified by mounted container -->' >> index.html && echo 'File modified on host'" \
  stdout
```

#### Comparing Copy vs. Mount

| Feature | `with-directory` (Copy) | `with-mounted-directory` (Mount) |
|---------|-------------------------|----------------------------------|
| Changes affect host | No | Yes |
| Performance | Slower (needs to copy) | Faster (direct access) |
| Isolation | Better | Limited |
| Use case | Build artifacts, transformations | Development, persistent changes |
| File ownership | Container's user | Host's user |

### Executing Commands

Execute a simple command:

```bash
dagger core container from --address="alpine:latest" \
  with-directory --path="/workspace" --directory="." \
  with-workdir --path="/workspace" \
  with-exec --args="ls","-lah" \
  stdout
```

Execute a complex command with shell:

```bash
dagger core container from --address="alpine:latest" \
  with-directory --path="/workspace" --directory="." \
  with-workdir --path="/workspace" \
  with-exec --args="sh","-c","echo 'Files:' && ls -lah && echo 'Content:' && cat index.html | head -5" \
  stdout
```

### Modifying Files in the Container

This example demonstrates modifying files within the container:

```bash
dagger core container from --address="alpine:latest" \
  with-directory --path="/workspace" --directory="." \
  with-workdir --path="/workspace" \
  with-exec --args="sh","-c","echo '<!-- Modified in container -->' >> index.html && cat index.html | head -10" \
  stdout
```

## Advanced Usage

### Using Different Base Images

Node.js example:

```bash
dagger core container from --address="node:18-alpine" \
  with-directory --path="/app" --directory="." \
  with-workdir --path="/app" \
  with-exec --args="sh","-c","npm install && npm run build" \
  stdout
```

Python example:

```bash
dagger core container from --address="python:3.9-slim" \
  with-directory --path="/app" --directory="." \
  with-workdir --path="/app" \
  with-exec --args="sh","-c","pip install -r requirements.txt && python app.py" \
  stdout
```

### Running a Web Server

```bash
dagger core container from --address="node:18-alpine" \
  with-directory --path="/app" --directory="." \
  with-workdir --path="/app" \
  with-exposed-port --port=8080 \
  with-exec --args="npx","http-server","-p","8080" \
  as-service \
  up --ports=8080:8080
```

### Building and Exporting

Build and export a container:

```bash
dagger core container from --address="node:18-alpine" \
  with-directory --path="/app" --directory="." \
  with-workdir --path="/app" \
  with-exec --args="npm","install" \
  with-exec --args="npm","run","build" \
  export --path="./app-build"
```

## Common Patterns

### Development Environment

Set up a development environment with all dependencies:

```bash
dagger core container from --address="golang:1.20" \
  with-directory --path="/go/src/app" --directory="." \
  with-workdir --path="/go/src/app" \
  with-exec --args="go","mod","download" \
  terminal
```

### Testing

Run tests in an isolated environment:

```bash
dagger core container from --address="python:3.9" \
  with-directory --path="/app" --directory="." \
  with-workdir --path="/app" \
  with-exec --args="pip","install","-r","requirements.txt" \
  with-exec --args="pytest" \
  stdout
```

### Multi-stage Build

Example of a multi-stage build:

```bash
# Build stage
dagger core container from --address="node:18" \
  with-directory --path="/app" --directory="." \
  with-workdir --path="/app" \
  with-exec --args="npm","install" \
  with-exec --args="npm","run","build" \
  directory --path="/app/build" > build.tar

# Runtime stage
dagger core container from --address="nginx:alpine" \
  with-directory --path="/usr/share/nginx/html" --directory=build.tar \
  publish --address="my-registry/my-app:latest"
```

## Tips and Best Practices

1. **Use Copy-on-Write** when you want to modify files without affecting the host
2. **Use Mounting** when you need changes to persist back to the host
3. **Cache Dependencies** using `with-mounted-cache` for faster builds
4. **Use Multi-stage Builds** to keep final images small
5. **Export Results** using `export` or `publish` to save artifacts
6. **Use `stdout` and `stderr`** to capture command output
7. **Set Working Directory** with `with-workdir` for cleaner commands

## Copy-on-Write vs. Mounting: Real-world Use Cases

### When to Use Copy-on-Write (`with-directory`)

1. **CI/CD Pipelines**: When building artifacts in a continuous integration pipeline where you want isolation and don't need changes to persist.

   ```bash
   dagger core container from --address="maven:3.8-openjdk-11" \
     with-directory --path="/app" --directory="." \
     with-workdir --path="/app" \
     with-exec --args="mvn","clean","package" \
     directory --path="/app/target" > artifacts.tar
   ```

2. **Parallel Processing**: When you need to process the same files in different ways simultaneously.

3. **Temporary Transformations**: When you need to modify files for a specific process but want to keep the original intact.

4. **Security Sensitive Operations**: When you want to ensure that container operations cannot modify your source code.

### When to Use Mounting (`with-mounted-directory`)

1. **Development Workflows**: When you're actively developing and want changes to be immediately reflected in your source code.

   ```bash
   dagger core container from --address="node:18" \
     with-mounted-directory --path="/app" --directory="." \
     with-workdir --path="/app" \
     with-exec --args="npm","run","dev" \
     terminal
   ```

2. **File Generation**: When you want the container to generate files that should persist on the host.

   ```bash
   dagger core container from --address="openapi-generator-cli" \
     with-mounted-directory --path="/input" --directory="./api-specs" \
     with-mounted-directory --path="/output" --directory="./generated-code" \
     with-exec --args="generate","-i","/input/openapi.yaml","-g","typescript-axios","-o","/output" \
     stdout
   ```

3. **Database Operations**: When working with databases that need to persist data between runs.

4. **Caching**: When you want to cache build artifacts between runs for faster builds.

   ```bash
   dagger core container from --address="gradle:7.4-jdk11" \
     with-directory --path="/app" --directory="." \
     with-mounted-directory --path="/app/.gradle" --directory="./.gradle-cache" \
     with-workdir --path="/app" \
     with-exec --args="gradle","build" \
     stdout
   ```

### Hybrid Approach

Sometimes, a hybrid approach works best:

```bash
# Mount the source code for development, but use copy-on-write for dependencies
dagger core container from --address="node:18" \
  with-mounted-directory --path="/app/src" --directory="./src" \
  with-directory --path="/app" --directory="." --exclude=["src/**"] \
  with-mounted-directory --path="/app/node_modules" --directory="./node_modules" \
  with-workdir --path="/app" \
  with-exec --args="npm","run","dev" \
  terminal
```

## Copy-on-Write Workflows

Copy-on-write is particularly useful for several workflows:

### 1. Build and Transform

Copy files into the container, transform them, and export the results:

```bash
dagger core container from --address="node:18" \
  with-directory --path="/app" --directory="." \
  with-workdir --path="/app" \
  with-exec --args="npm","install" \
  with-exec --args="npm","run","build" \
  directory --path="/app/dist" > build.tar
```

### 2. Test Without Side Effects

Run tests that might modify files without affecting your source code:

```bash
dagger core container from --address="python:3.9" \
  with-directory --path="/app" --directory="." \
  with-workdir --path="/app" \
  with-exec --args="sh","-c","pip install -r requirements.txt && python -m pytest --modify-files" \
  stdout
```

### 3. Multiple Parallel Transformations

Process the same source files in different ways simultaneously:

```bash
# Container 1: Build for production
dagger core container from --address="node:18" \
  with-directory --path="/app" --directory="." \
  with-workdir --path="/app" \
  with-exec --args="sh","-c","npm install && npm run build:prod" \
  directory --path="/app/dist" > prod.tar

# Container 2: Build for development
dagger core container from --address="node:18" \
  with-directory --path="/app" --directory="." \
  with-workdir --path="/app" \
  with-exec --args="sh","-c","npm install && npm run build:dev" \
  directory --path="/app/dist" > dev.tar
```

### 4. Temporary Modifications

Make temporary changes for testing or processing:

```bash
dagger core container from --address="alpine:latest" \
  with-directory --path="/workspace" --directory="." \
  with-workdir --path="/workspace" \
  with-exec --args="sh","-c","sed -i 's/development/production/g' config.json && ./run-tests.sh" \
  stdout
```

## Troubleshooting

- If a command fails, check the exit code and stderr output
- Use `terminal` to get an interactive shell for debugging
- Add `-v` or `-vv` to the dagger command for verbose output
- Use `sync` to force evaluation of the pipeline for debugging

## Reference

- [Dagger Documentation](https://docs.dagger.io/)
- [Dagger CLI Reference](https://docs.dagger.io/cli/reference/)