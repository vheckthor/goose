# Goose Benchmarking Framework

The `goose-bench` crate provides a framework for benchmarking and evaluating LLM models with the Goose framework. This tool helps quantify model performance across various tasks and generate structured reports.

## Features

- Run benchmark suites across multiple LLM models
- Execute evaluations in parallel when supported
- Generate structured JSON and CSV reports
- Process evaluation results with custom scripts
- Calculate aggregate metrics across evaluations
- Support for tool-shim evaluation

## Configuration

Benchmark configuration is provided through a JSON file. Here's a sample configuration file (leaderboard-config.json) that you can use as a template:

```json
{
  "models": [
    {
      "provider": "databricks",
      "name": "gpt-4-1-mini",
      "parallel_safe": true,
      "tool_shim": {
        "use_tool_shim": false,
        "tool_shim_model": null
      }
    },
    {
      "provider": "databricks",
      "name": "gpt-4-1-mini",
      "parallel_safe": true,
      "tool_shim": null
    },
    {
      "provider": "databricks",
      "name": "gpt-4-1-mini",
      "parallel_safe": true,
      "tool_shim": null
    },
    {
      "provider": "databricks",
      "name": "gpt-4-1-2025-04-14",
      "parallel_safe": true,
      "tool_shim": null
    },
    {
      "provider": "databricks",
      "name": "claude-3-5-sonnet",
      "parallel_safe": true,
      "tool_shim": null
    },
    {
      "provider": "databricks",
      "name": "claude-3-5-haiku",
      "parallel_safe": true,
      "tool_shim": null
    },
    {
      "provider": "databricks",
      "name": "gpt-4o",
      "parallel_safe": true,
      "tool_shim": null
    }
  ],
  "evals": [
    {
      "selector": "core:developer",
      "post_process_cmd": null,
      "parallel_safe": true
    },
    {
      "selector": "core:developer_search_replace",
      "post_process_cmd": null,
      "parallel_safe": true
    },
    {
      "selector": "vibes:blog_summary",
      "post_process_cmd": "/Users/ahau/Development/goose-1.0/goose/scripts/bench-postprocess-scripts/run_vibes_judge.sh",
      "parallel_safe": true
    },
    {
      "selector": "vibes:flappy_bird",
      "post_process_cmd": null,
      "parallel_safe": true
    },
    {
      "selector": "vibes:goose_wiki",
      "post_process_cmd": null,
      "parallel_safe": true
    },
    {
      "selector": "vibes:restaurant_research",
      "post_process_cmd": "/Users/ahau/Development/goose-1.0/goose/scripts/bench-postprocess-scripts/run_vibes_judge.sh",
      "parallel_safe": true
    },
    {
      "selector": "vibes:squirrel_census",
      "post_process_cmd": null,
      "parallel_safe": true
    }
  ],
  "include_dirs": [],
  "repeat": 3,
  "run_id": null,
  "output_dir": "/path/to/output/directory",
  "eval_result_filename": "eval-results.json",
  "run_summary_filename": "run-results-summary.json",
  "env_file": "/path/to/.goosebench.env"
}
```

## Configuration Options

### Models

- `provider`: The LLM provider (e.g., "databricks", "openai")
- `name`: The model name
- `parallel_safe`: Whether the model can be run in parallel
- `tool_shim`: Configuration for tool-shim support
  - `use_tool_shim`: Whether to use tool-shim
  - `tool_shim_model`: Optional custom model for tool-shim

### Evaluations

- `selector`: The evaluation selector in format `suite:evaluation`
- `post_process_cmd`: Optional path to a post-processing script
- `parallel_safe`: Whether the evaluation can be run in parallel

### Global Configuration

- `include_dirs`: Additional directories to include in the benchmark environment
- `repeat`: Number of times to repeat evaluations (for statistical significance)
- `run_id`: Optional identifier for the run (defaults to timestamp)
- `output_dir`: Directory to store benchmark results (must be absolute path)
- `eval_result_filename`: Filename for individual evaluation results
- `run_summary_filename`: Filename for run summary
- `env_file`: Optional path to environment variables file

## Post-Processing

You can specify post-processing commands for evaluations, which will be executed after each evaluation completes. The command receives the path to the evaluation results file as its first argument.

For example, the `run_vibes_judge.sh` script processes outputs from the `blog_summary` and `restaurant_research` evaluations, using LLM-based judging to assign scores.

## CSV Report Generation

The framework automatically generates CSV reports with aggregate metrics, making it easy to analyze and compare model performances across different evaluations.

## Environment Variables

You can provide environment variables through the `env_file` configuration option. This is useful for provider API keys and other sensitive information.

## Running Benchmarks

Use the Goose CLI to run benchmarks with your configuration:

```bash
goose bench -c /path/to/leaderboard-config.json
```

## Output Structure

Results are organized in a directory structure that follows this pattern:

```
{output_dir}/
└── {provider}-{model}/
    ├── eval-results/
    │   ├── aggregate_metrics.csv
    │   ├── suite_evaluation1.csv
    │   └── suite_evaluation2.csv
    └── run-{run_id}/
        ├── {suite}/
        │   └── {evaluation}/
        │       └── eval-results.json
        └── run-results-summary.json
```

Each model gets its own directory, containing run results and aggregated CSV files for analysis.