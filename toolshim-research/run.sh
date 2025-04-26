#!/bin/sh

rm -rf out-chat out-listing out-create-file out-create-run
mkdir -p out-chat out-listing out-create-file out-create-run
# 
./ollama_tester.py --query "what can you do?" --output "out-chat"
./ollama_tester.py --query "list files in this dir" --output "out-listing"
./ollama_tester.py --query "can you make a file hello.py and put a simple hello world app in it" --output "out-create-file"
./ollama_tester.py --query "can you make a file boop.js and put a simple hello world app in it, and then run it with node" --output "out-create-run"
./ollama_tester.py --query "what is in this photo pic.png" --output "out-create-image" 
./ollama_tester.py --query "can you tell me what windows I have open" --output "out-create-windows" 
./ollama_tester.py --query "I would like you to take a screenshot and tell me what you see" --output "out-create-screenshot" 
