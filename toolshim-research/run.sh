#!/bin/sh

rm -rf out-chat out-listing
mkdir -p out-chat out-listing
# 
./ollama_tester.py --query "what can you do?" --output "out-chat"
./ollama_tester.py --query "list files in this dir" --output "out-listing"
