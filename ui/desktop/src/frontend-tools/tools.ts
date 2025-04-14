import { FrontendToolDefinition } from './types';
import { toolRegistry } from './registry';

// Calculator tool definition
export const calculatorTool: FrontendToolDefinition = {
  name: 'calculator',
  description: 'Perform basic arithmetic calculations',
  inputSchema: {
    type: 'object',
    required: ['operation', 'numbers'],
    properties: {
      operation: {
        type: 'string',
        description: 'The arithmetic operation to perform',
        enum: ['add', 'subtract', 'multiply', 'divide'],
      },
      numbers: {
        type: 'array',
        description: 'List of numbers to operate on in order',
        items: {
          type: 'number',
        },
      },
    },
  },
};

// Calculator implementation
async function executeCalculator(args: Record<string, unknown>) {
  const operation = args.operation as string;
  const numbers = (args.numbers as number[]) || [];

  if (numbers.length === 0) {
    throw new Error('No numbers provided');
  }

  let result: number;
  switch (operation) {
    case 'add':
      result = numbers.reduce((a, b) => a + b, 0);
      break;
    case 'subtract':
      result = numbers[0] - numbers.slice(1).reduce((a, b) => a + b, 0);
      break;
    case 'multiply':
      result = numbers.reduce((a, b) => a * b, 1);
      break;
    case 'divide':
      result = numbers.reduce((a, b) => {
        if (b === 0) throw new Error('Division by zero');
        return a / b;
      });
      break;
    default:
      throw new Error(`Unknown operation: ${operation}`);
  }

  return [
    {
      type: 'text',
      text: result.toString(),
      annotations: null,
    },
  ];
}

// Register calculator tool
toolRegistry.registerTool(calculatorTool, executeCalculator);

// Export available tools
export const availableTools = [calculatorTool];
