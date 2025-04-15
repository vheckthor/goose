#!/bin/sh

rm -rf out-chat out-listing out-create-file out-create-run
mkdir -p out-chat out-listing out-create-file out-create-run
# 
./ollama_tester.py --query "what can you do?" --output "out-chat"
./ollama_tester.py --query "list files in this dir" --output "out-listing"
./ollama_tester.py --query "can you make a file hello.py and put a simple hello world app in it" --output "out-create-file"
./ollama_tester.py --query "can you make a file boop.js and put a simple hello world app in it, and then run it with node" --output "out-create-run"
