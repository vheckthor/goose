export interface Prompt {
  id: string;
  title: string;
  description: string;
  category: string;
  role: string;
  prompt: string;
  example: string;
  tags: string[];
  author: string;
  dateAdded: string;
  lastUpdated: string;
  rating: number;
  usageCount: number;
  verified: boolean;
  extensions: string[];
  variables?: {
    name: string;
    description: string;
    required: boolean;
    type: string;
  }[];
}