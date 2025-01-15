import TurndownService from 'turndown';

// Initialize turndown once
const turndownService = new TurndownService({
  headingStyle: 'atx',
  codeBlockStyle: 'fenced',
  emDelimiter: '*'
});

export function htmlToMarkdown(html: string): string {
  return turndownService.turndown(html);
}