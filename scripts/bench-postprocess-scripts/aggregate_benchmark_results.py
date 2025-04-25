#!/usr/bin/env python3
"""
Aggregate benchmark results from multiple benchmark directories.

This script processes benchmark results by:
1. Finding all eval-results/aggregate_metrics.csv files in subdirectories of all provided benchmark directories
2. Grouping by provider and model_name
3. Averaging score_mean, prompt_execution_time_seconds_mean, and total_tool_calls_mean
4. Creating a unified table and saving it as a CSV
"""

import argparse
import pandas as pd
from pathlib import Path
import sys
from typing import List


def find_aggregate_metrics_files(benchmark_dirs: List[Path]) -> List[Path]:
    """Find all aggregate_metrics.csv files in the benchmark directories."""
    csv_files = []
    
    for benchmark_dir in benchmark_dirs:
        # Look for eval-results/aggregate_metrics.csv in each subdirectory
        for subdir in benchmark_dir.iterdir():
            if subdir.is_dir():
                csv_path = subdir / "eval-results" / "aggregate_metrics.csv"
                if csv_path.exists():
                    csv_files.append(csv_path)
    
    return csv_files


def process_csv_files(csv_files: List[Path]) -> pd.DataFrame:
    """Process all CSV files and aggregate the results."""
    all_data = []
    
    for csv_file in csv_files:
        try:
            df = pd.read_csv(csv_file)
            
            # Check if required columns exist
            required_columns = [
                'provider', 'model_name', 'score_mean', 
                'prompt_execution_time_seconds_mean', 'total_tool_calls_mean', 'total_tokens_mean'
            ]
            
            missing_columns = [col for col in required_columns if col not in df.columns]
            if missing_columns:
                print(f"Warning: {csv_file} is missing columns: {missing_columns}")
                continue
            
            # Select only the required columns
            df_subset = df[required_columns]
            all_data.append(df_subset)
            
        except Exception as e:
            print(f"Error processing {csv_file}: {str(e)}")
    
    if not all_data:
        raise ValueError("No valid CSV files found with required columns")
    
    # Concatenate all dataframes
    combined_df = pd.concat(all_data, ignore_index=True)
    
    # Group by provider and model_name, then calculate averages
    aggregated_df = combined_df.groupby(['provider', 'model_name']).agg({
        'score_mean': 'mean',
        'prompt_execution_time_seconds_mean': 'mean',
        'total_tool_calls_mean': 'mean',
        'total_tokens_mean': 'mean'
    }).reset_index()
    
    # Rename columns to indicate they are averages
    aggregated_df.columns = [
        'provider', 'model_name', 
        'avg_score_mean', 'avg_prompt_execution_time_seconds_mean', 
        'avg_total_tool_calls_mean',
        'avg_total_tokens_mean'
    ]
    
    # Sort by provider and model_name for better readability
    aggregated_df = aggregated_df.sort_values(['provider', 'model_name'])
    
    return aggregated_df


def main():
    parser = argparse.ArgumentParser(
        description="Aggregate benchmark results from multiple benchmark directories"
    )
    parser.add_argument(
        "benchmark_dirs", 
        type=str, 
        nargs='+',
        help="Paths to one or more benchmark directories (e.g., /path/to/benchmark-2025-04-24-22:04:06)"
    )
    parser.add_argument(
        "--output", 
        type=str, 
        default="aggregated_benchmark_results.csv",
        help="Output CSV file name (default: aggregated_benchmark_results.csv)"
    )
    
    args = parser.parse_args()
    
    # Convert paths to Path objects and validate they exist
    benchmark_dirs = []
    for dir_path in args.benchmark_dirs:
        path = Path(dir_path)
        if not path.exists():
            print(f"Error: Benchmark directory {path} does not exist")
            sys.exit(1)
        benchmark_dirs.append(path)
    
    try:
        # Find all aggregate_metrics.csv files across all benchmark directories
        csv_files = find_aggregate_metrics_files(benchmark_dirs)
        
        if not csv_files:
            print(f"No aggregate_metrics.csv files found in any of the provided directories")
            sys.exit(1)
        
        print(f"Found {len(csv_files)} aggregate_metrics.csv files across {len(benchmark_dirs)} directories")
        
        # Process and aggregate the data
        aggregated_df = process_csv_files(csv_files)
        
        # Save the results
        output_path = Path(args.output)
        aggregated_df.to_csv(output_path, index=False)
        
        print(f"\nAggregated results saved to: {output_path}")
        print("\nSummary:")
        print(aggregated_df.to_string(index=False))
        
    except Exception as e:
        print(f"Error: {str(e)}")
        sys.exit(1)


if __name__ == "__main__":
    main()