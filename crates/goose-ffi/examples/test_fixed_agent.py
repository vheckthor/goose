#!/usr/bin/env python3
"""
Test script for the fixed Goose FFI implementation
"""

import os
import sys

# Set environment variables for testing
os.environ["DATABRICKS_API_KEY"] = "dummy_key_for_testing"
os.environ["DATABRICKS_HOST"] = "https://dummy-host.databricks.com"

# Import and run the fixed agent
import goose_agent_fixed

# Run the main function
goose_agent_fixed.main()