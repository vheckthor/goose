# Makefile for building Goose FFI examples

# Platform-specific settings
ifeq ($(shell uname -s),Darwin)
    LIB_NAME := libgoose_ffi.dylib
else ifeq ($(shell uname -s),Linux)
    LIB_NAME := libgoose_ffi.so
else
    LIB_NAME := goose_ffi.dll
endif

# Build directories
ROOT_DIR := ..
TARGET_DIR := $(ROOT_DIR)/target/debug
INCLUDE_DIR := $(ROOT_DIR)/include

# Compiler settings
CC := gcc
CFLAGS := -g -Wall -I$(INCLUDE_DIR)
LDFLAGS := -L$(TARGET_DIR) -lgoose_ffi

# Targets
.PHONY: all clean build-lib tool-agent run-tool-agent

all: build-lib tool-agent

# Build the Rust library
build-lib:
	@echo "Building Goose FFI library..."
	cd $(ROOT_DIR) && cargo build

# Build the C tool agent example
tool-agent: build-lib tool_agent.c
	@echo "Building tool_agent..."
	$(CC) $(CFLAGS) -o tool_agent tool_agent.c $(LDFLAGS)

# Run the C tool agent example
run-tool-agent: tool-agent
	@echo "Running tool_agent..."
	@# Set LD_LIBRARY_PATH for Linux or DYLD_LIBRARY_PATH for macOS
	@if [ "$(shell uname -s)" = "Darwin" ]; then \
		DYLD_LIBRARY_PATH=$(TARGET_DIR) ./tool_agent; \
	else \
		LD_LIBRARY_PATH=$(TARGET_DIR) ./tool_agent; \
	fi

# Run the Python tool agent example
run-python-tool-agent: build-lib
	@echo "Running Python tool agent..."
	@# Set PYTHONPATH and LD_LIBRARY_PATH/DYLD_LIBRARY_PATH
	@if [ "$(shell uname -s)" = "Darwin" ]; then \
		DYLD_LIBRARY_PATH=$(TARGET_DIR) python3 tool_agent.py; \
	else \
		LD_LIBRARY_PATH=$(TARGET_DIR) python3 tool_agent.py; \
	fi

# Clean up
clean:
	rm -f tool_agent
	@echo "Cleaned up example binaries"