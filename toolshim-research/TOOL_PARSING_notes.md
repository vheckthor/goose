# XML Tool Call Parser

This document describes how Cline parses XML-formatted tool calls from LLM responses and how it instructs LLMs to use this format. The implementation can be found in `src/core/assistant-message/parse-assistant-message.ts`.

## XML Tool Call Format

Cline uses a custom XML-based format for tool calls:

```xml
<tool_name>
<parameter1_name>value1</parameter1_name>
<parameter2_name>value2</parameter2_name>
...
</tool_name>
```

Example:
```xml
<execute_command>
<command>npm install</command>
<requires_approval>true</requires_approval>
</execute_command>
```

## Instructing LLMs to Use XML Format

Cline instructs LLMs to use the XML format through its system prompt, which is defined in `src/core/prompts/system.ts`. The system prompt includes:

1. **Format Definition**: Clear explanation of the XML-style tag structure
   ```
   Tool use is formatted using XML-style tags. The tool name is enclosed in opening and closing tags, and each parameter is similarly enclosed within its own set of tags.
   ```

2. **Examples**: Concrete examples of properly formatted tool calls for each available tool
   ```
   <execute_command>
   <command>npm run dev</command>
   <requires_approval>false</requires_approval>
   </execute_command>
   ```

3. **Tool Use Guidelines**: Explicit instructions to use the XML format
   ```
   Formulate your tool use using the XML format specified for each tool.
   ```

4. **Error Handling**: When a tool call is not properly formatted, Cline sends reminders about the correct format (defined in `src/core/prompts/responses.ts`)
   ```
   # Reminder: Instructions for Tool Use
   
   Tool uses are formatted using XML-style tags...
   ```

5. **Model-Specific Formatting**: For models like O1 that need special handling, Cline uses transformers (in `src/api/transform/o1-format.ts`) to convert between different formats while maintaining the XML structure in the user-facing interface.

## Parser Pseudocode

Below is a pseudocode representation of the parsing mechanism that can be adapted for implementation in any programming language:

```
function parseAssistantMessage(assistantMessage):
    // Initialize data structures
    contentBlocks = []                  // Will hold all parsed content blocks
    currentTextContent = null           // Current text content being accumulated
    currentTextContentStartIndex = 0    // Start index of current text content
    currentToolUse = null               // Current tool call being accumulated
    currentToolUseStartIndex = 0        // Start index of current tool call
    currentParamName = null             // Current parameter name being processed
    currentParamValueStartIndex = 0     // Start index of current parameter value
    accumulator = ""                    // Character accumulator
    
    // Tool and parameter names are defined in src/core/assistant-message/index.ts
    toolUseNames = ["execute_command", "read_file", "write_to_file", ...]
    toolParamNames = ["command", "path", "content", ...]
    
    // Process the message character by character
    for i = 0 to length(assistantMessage) - 1:
        char = assistantMessage[i]
        accumulator += char
        
        // Case 1: Inside a parameter of a tool call
        if currentToolUse != null and currentParamName != null:
            currentParamValue = accumulator.substring(currentParamValueStartIndex)
            paramClosingTag = "</" + currentParamName + ">"
            
            if currentParamValue.endsWith(paramClosingTag):
                // Extract parameter value without the closing tag
                value = currentParamValue.substring(0, length(currentParamValue) - length(paramClosingTag)).trim()
                currentToolUse.params[currentParamName] = value
                currentParamName = null
                continue
            else:
                // Still accumulating parameter value
                continue
        
        // Case 2: Inside a tool call but not inside a parameter
        if currentToolUse != null:
            currentToolValue = accumulator.substring(currentToolUseStartIndex)
            toolUseClosingTag = "</" + currentToolUse.name + ">"
            
            if currentToolValue.endsWith(toolUseClosingTag):
                // Tool call is complete
                currentToolUse.partial = false
                contentBlocks.append(currentToolUse)
                currentToolUse = null
                continue
            else:
                // Check if we're starting a new parameter
                possibleParamOpeningTags = map(toolParamNames, name -> "<" + name + ">")
                
                for paramOpeningTag in possibleParamOpeningTags:
                    if accumulator.endsWith(paramOpeningTag):
                        // Start of a new parameter
                        currentParamName = paramOpeningTag.substring(1, length(paramOpeningTag) - 1)
                        currentParamValueStartIndex = length(accumulator)
                        break
                
                // Special case for write_to_file content parameter
                // Handles nested tags within content
                if currentToolUse.name == "write_to_file" and accumulator.endsWith("</content>"):
                    toolContent = accumulator.substring(currentToolUseStartIndex)
                    contentStartTag = "<content>"
                    contentEndTag = "</content>"
                    contentStartIndex = toolContent.indexOf(contentStartTag) + length(contentStartTag)
                    contentEndIndex = toolContent.lastIndexOf(contentEndTag)
                    
                    if contentStartIndex != -1 and contentEndIndex != -1 and contentEndIndex > contentStartIndex:
                        currentToolUse.params["content"] = toolContent.substring(contentStartIndex, contentEndIndex).trim()
                
                // Still accumulating tool call
                continue
        
        // Case 3: Not inside a tool call
        didStartToolUse = false
        possibleToolUseOpeningTags = map(toolUseNames, name -> "<" + name + ">")
        
        for toolUseOpeningTag in possibleToolUseOpeningTags:
            if accumulator.endsWith(toolUseOpeningTag):
                // Start of a new tool call
                toolName = toolUseOpeningTag.substring(1, length(toolUseOpeningTag) - 1)
                currentToolUse = {
                    type: "tool_use",
                    name: toolName,
                    params: {},
                    partial: true
                }
                currentToolUseStartIndex = length(accumulator)
                
                // Finalize any current text content
                if currentTextContent != null:
                    currentTextContent.partial = false
                    // Remove partial tool tag from text content
                    currentTextContent.content = currentTextContent.content
                        .substring(0, length(currentTextContent.content) - length(toolUseOpeningTag) + 1).trim()
                    contentBlocks.append(currentTextContent)
                    currentTextContent = null
                
                didStartToolUse = true
                break
        
        if not didStartToolUse:
            // Accumulating text content
            if currentTextContent == null:
                currentTextContentStartIndex = i
            
            currentTextContent = {
                type: "text",
                content: accumulator.substring(currentTextContentStartIndex).trim(),
                partial: true
            }
    
    // Handle any partial content at the end of the message
    if currentToolUse != null:
        // Stream ended during tool call
        if currentParamName != null:
            // Stream ended during parameter
            currentToolUse.params[currentParamName] = accumulator.substring(currentParamValueStartIndex).trim()
        
        contentBlocks.append(currentToolUse)
    
    if currentTextContent != null:
        // Stream ended during text content
        contentBlocks.append(currentTextContent)
    
    return contentBlocks
```

## Implementation Notes

1. **Predefined Tool Names and Parameters**: 
   - Defined in `src/core/assistant-message/index.ts`
   - Should be adapted to your application's supported tools

2. **Content Types**:
   - The parser produces two types of content: text content and tool calls
   - Each is represented with a distinct data structure as defined in `src/core/assistant-message/index.ts`

3. **Streaming Support**:
   - The `partial` flag indicates incomplete content during streaming
   - This allows for progressive rendering of responses

4. **Special Cases**:
   - The special handling for `write_to_file` is important for correctly parsing file content that may contain XML-like tags

5. **Integration**:
   - This parser should be applied to normalized responses from LLM providers
   - See provider implementations in `src/api/providers/` for examples of response normalization

## Key Design Considerations

1. **Character-by-Character Processing**:
   - The parser processes the input string character by character rather than using regex or a standard XML parser
   - This approach provides more control over the parsing process and better handles streaming responses

2. **State Machine Approach**:
   - The parser implements a simple state machine to track the current parsing context
   - States include: text content, tool call, parameter name, parameter value

3. **Edge Case Handling**:
   - Special handling for nested tags within content parameters
   - Support for partial/incomplete responses during streaming

4. **Performance Considerations**:
   - The character-by-character approach is efficient for streaming responses
   - The parser avoids unnecessary string copies by using substring operations

## Provider-Specific Handling

Different LLM providers may have different native formats for tool calls. Cline handles this through provider-specific transformers:

1. **OpenAI**: Uses function calling format natively, which Cline transforms to/from XML format
2. **Anthropic**: Uses tool_use blocks which align well with Cline's XML format
3. **O1**: Requires special handling via `src/api/transform/o1-format.ts` which includes XML format instructions in the prompt

The XML format serves as a common interface for all providers, ensuring consistent parsing and execution regardless of the underlying LLM.