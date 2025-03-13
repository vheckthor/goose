import type { Prompt } from "./types/prompt";

// Import all prompt JSON files
import codeReviewTemplate from "../prompts/roles/developer/code-review-template.json";
import dataAnalysisHelper from "../prompts/roles/data-scientist/data-analysis-helper.json";

// Create a static array of all prompts
const promptsData: Prompt[] = [
  codeReviewTemplate,
  dataAnalysisHelper
];

export async function fetchPrompts(): Promise<Prompt[]> {
  return promptsData;
}

export async function searchPrompts(query: string): Promise<Prompt[]> {
  const lowercaseQuery = query.toLowerCase();
  
  return promptsData.filter(prompt => 
    prompt.title.toLowerCase().includes(lowercaseQuery) ||
    prompt.description.toLowerCase().includes(lowercaseQuery) ||
    prompt.category.toLowerCase().includes(lowercaseQuery) ||
    prompt.function.toLowerCase().includes(lowercaseQuery) ||
    prompt.tags.some(tag => tag.toLowerCase().includes(lowercaseQuery))
  );
}