import type { Prompt } from "./types/prompt";

// This would typically fetch from an API or database
export async function fetchPrompts(): Promise<Prompt[]> {
  // Example data - replace with actual data source
  return [
    {
      id: "dev-code-review",
      title: "Code Review Assistant",
      description: "Help developers review code changes with detailed analysis and suggestions",
      category: "Development",
      function: "coding",
      prompt: `As a code review assistant, analyze the following code changes:

{code_diff}

Please provide:
1. A high-level summary of changes
2. Potential issues or concerns
3. Style and best practice suggestions
4. Security considerations
5. Performance implications

Focus on being constructive and specific in your feedback.`,
      example: "Here's an example code review response...",
      tags: ["development", "code-review", "security", "best-practices"],
      author: "GooseAI Team",
      dateAdded: "2025-01-15",
      lastUpdated: "2025-03-01",
      rating: 4.8,
      usageCount: 1250,
      verified: true,
      extensions: ["developer", "git"], // Updated to use friendly names
      variables: [
        {
          name: "code_diff",
          description: "The code changes to review, typically in diff format",
          required: true,
          type: "text"
        }
      ]
    },
    {
      id: "data-analysis",
      title: "Data Analysis Helper",
      description: "Guide for analyzing data sets and creating visualizations",
      category: "Data Science",
      function: "data",
      prompt: `As a data analysis assistant, help analyze the following dataset:

{dataset_description}

Please:
1. Suggest relevant analysis methods
2. Recommend appropriate visualizations
3. Identify potential insights
4. Point out data quality issues
5. Propose next steps

Consider the business context and goals in your analysis.`,
      example: "Example analysis of a sales dataset...",
      tags: ["data-analysis", "visualization", "statistics"],
      author: "Data Science Team",
      dateAdded: "2025-02-01",
      lastUpdated: "2025-03-10",
      rating: 4.7,
      usageCount: 850,
      verified: true,
      extensions: ["computercontroller"],
      variables: [
        {
          name: "dataset_description",
          description: "Description of the dataset including its structure and context",
          required: true,
          type: "text"
        }
      ]
    }
  ];
}

export async function searchPrompts(query: string): Promise<Prompt[]> {
  const prompts = await fetchPrompts();
  const lowercaseQuery = query.toLowerCase();
  
  return prompts.filter(prompt => 
    prompt.title.toLowerCase().includes(lowercaseQuery) ||
    prompt.description.toLowerCase().includes(lowercaseQuery) ||
    prompt.category.toLowerCase().includes(lowercaseQuery) ||
    prompt.function.toLowerCase().includes(lowercaseQuery) ||
    prompt.tags.some(tag => tag.toLowerCase().includes(lowercaseQuery))
  );
}