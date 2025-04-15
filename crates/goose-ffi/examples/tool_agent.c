/**
 * Tool Agent Example
 * 
 * This example demonstrates how to use the Goose FFI interface to create an
 * agent that can invoke tools.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../include/goose_ffi.h"

// Calculator tool callback that performs arithmetic operations
char* calculator_tool(size_t param_count, const goose_ToolParam* params, void* user_data) {
    double a = 0.0;
    double b = 0.0;
    char* operation = NULL;
    
    // Extract parameters
    for (size_t i = 0; i < param_count; i++) {
        const goose_ToolParam* param = &params[i];
        
        if (strcmp(param->name, "a") == 0) {
            // Parse 'a' parameter as double
            sscanf(param->value, "%lf", &a);
        } 
        else if (strcmp(param->name, "b") == 0) {
            // Parse 'b' parameter as double
            sscanf(param->value, "%lf", &b);
        }
        else if (strcmp(param->name, "operation") == 0) {
            // Parse operation parameter (remove quotes from JSON string)
            size_t len = strlen(param->value);
            if (len > 2) {
                operation = malloc(len - 1); // -1 for the quotes
                strncpy(operation, param->value + 1, len - 2);
                operation[len - 2] = '\0';
            }
        }
    }
    
    // Calculate result
    double result = 0.0;
    if (operation == NULL) {
        free(operation);
        return strdup("{\"error\": \"Missing operation parameter\"}");
    }
    
    if (strcmp(operation, "add") == 0) {
        result = a + b;
    } 
    else if (strcmp(operation, "subtract") == 0) {
        result = a - b;
    } 
    else if (strcmp(operation, "multiply") == 0) {
        result = a * b;
    } 
    else if (strcmp(operation, "divide") == 0) {
        if (b == 0) {
            free(operation);
            return strdup("{\"error\": \"Division by zero\"}");
        }
        result = a / b;
    } 
    else {
        free(operation);
        return strdup("{\"error\": \"Unknown operation\"}");
    }
    
    free(operation);
    
    // Allocate and format result as JSON
    char* result_str = malloc(100);
    sprintf(result_str, "{\"result\": %f}", result);
    return result_str;
}

int main() {
    // Create a provider configuration structure
    goose_ProviderConfigFFI config = {
        .provider_type = 0, // Databricks
        .api_key = NULL,    // Use environment variable
        .model_name = NULL, // Use default model
        .host = NULL        // Use environment variable
    };
    
    // Create an agent
    goose_AgentPtr agent = goose_agent_new(&config);
    if (agent._0 == NULL) {
        printf("Failed to create agent. Make sure DATABRICKS_API_KEY and DATABRICKS_HOST are set.\n");
        return 1;
    }
    
    printf("Agent created successfully.\n");
    
    // Create a calculator tool schema
    goose_ToolParamDef params[] = {
        {
            .name = "a",
            .description = "First number",
            .param_type = 1, // Number
            .required = 0    // Required
        },
        {
            .name = "b",
            .description = "Second number",
            .param_type = 1, // Number
            .required = 0    // Required
        },
        {
            .name = "operation",
            .description = "Operation to perform: add, subtract, multiply, or divide",
            .param_type = 0, // String
            .required = 0    // Required
        }
    };
    
    char* calculator_schema = goose_create_tool_schema(
        "calculator",
        "Perform arithmetic operations on two numbers",
        params,
        3 // Number of parameters
    );
    
    if (calculator_schema == NULL) {
        printf("Failed to create calculator schema.\n");
        goose_agent_free(agent);
        return 1;
    }
    
    printf("Calculator schema created successfully.\n");
    
    // Register the calculator tool with the agent
    bool success = goose_agent_register_tool_callback(
        agent,
        "calculator",
        "Perform arithmetic operations on two numbers",
        calculator_schema,
        calculator_tool,
        NULL // No user data for this example
    );
    
    // Free the schema string (we don't need it anymore)
    goose_free_string(calculator_schema);
    
    if (!success) {
        printf("Failed to register calculator tool.\n");
        goose_agent_free(agent);
        return 1;
    }
    
    printf("Calculator tool registered successfully.\n");
    
    // Prompt the user to provide instructions to the agent
    printf("\nYou can now ask the agent to perform calculations.\n");
    printf("Examples:\n");
    printf("- Calculate 5 + 3\n");
    printf("- What is 10 divided by 2?\n");
    printf("- Multiply 7 by 6\n\n");
    
    char input[1024];
    char* response;
    
    while (1) {
        printf("> ");
        if (fgets(input, sizeof(input), stdin) == NULL) {
            break;
        }
        
        // Remove newline
        input[strcspn(input, "\n")] = 0;
        
        // Check for exit command
        if (strcmp(input, "exit") == 0 || strcmp(input, "quit") == 0) {
            break;
        }
        
        // Send message to agent
        response = goose_agent_send_message(agent, input);
        
        if (response != NULL) {
            printf("Agent: %s\n\n", response);
            goose_free_string(response);
        } else {
            printf("Error: Failed to get response from agent.\n\n");
        }
    }
    
    // Free the agent
    goose_agent_free(agent);
    
    return 0;
}