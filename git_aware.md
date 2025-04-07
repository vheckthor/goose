### Git Integration Features

Want to add a tool to the developer mcp (in mod.rs) in goose-mcp, its job is to be repo aware and support checkpoints and actions to protect users changes. 

It should be conditional on env var GOOSE_GIT_CHECKPOINTING - feature flag, if that isn't set to true, then the tools should not appear at all in the set of tools in that MCP. 

General functionality: 
Know if dir operations are in is a git repo, if not, offer to make one. Know if there are unstaged changes the first time when starting and alert user. 
When commencing a task, it should make a git branch once it is in a clean state and no unstaged changes. It should be able to commit changes to that branch at logical checkpoints (which are either obvious or ask the user)

When working on a new task, can shift to a new branch with a reasonable name and work on that. 
Be aware of the branches it made, and offer to shift or roll back as needed. 
Should check with user if not sure. 

This idea is not to loose changes, that you or user makes. 
Also to be able to undo things to logical checkpoints (when things are in good state, or working) so if things go back, it can reset to the last good branch or commit on that branch. 


todo: 

in developer/mod.rs, consider and implement a new tool or 2 for making branch, commit, rolling back etc with clear instructions to the system of when and how to use (conditional on the feature flag var)