import { Message, createUserMessage, createAssistantMessage } from './types/message';

export interface SharedSessionDetails {
  share_token: string;
  created_at: number;
  base_url: string;
  description: string;
  messages: Message[];
}

/**
 * Fetches details for a specific shared session
 * @param baseUrl The base URL for session sharing API
 * @param shareToken The share token of the session to fetch
 * @returns Promise with shared session details
 */
export async function fetchSharedSessionDetails(
  baseUrl: string,
  shareToken: string
): Promise<SharedSessionDetails> {
  try {
    const response = await fetch(`${baseUrl}/sessions/share/${shareToken}`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
      credentials: 'include',
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch shared session: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();

    if (baseUrl != data.base_url) {
      throw new Error(`Base URL mismatch: ${baseUrl} != ${data.base_url}`);
    }

    return {
      share_token: data.share_token,
      created_at: data.created_at,
      base_url: data.base_url,
      description: data.description || 'Shared Session',
      messages: data.messages,
    };
  } catch (error) {
    console.error('Error fetching shared session:', error);
    throw error;
  }
}

/**
 * Fetches details for a specific shared session
 * @param baseUrl The base URL for session sharing API
 * @param shareToken The share token of the session to fetch
 * @returns Promise with shared session details
 */
export async function fetchMockedSharedSessionDetails(
  baseUrl: string,
  shareToken: string
): Promise<SharedSessionDetails> {
  return {
    share_token: '6dc69efc-5599-4493-968e-8615a93dd780',
    created_at: 1741874897,
    base_url: 'https://goosed.stage.sqprod.co/api',
    description: 'Mocked Shared Session',
    messages: [
      {
        created: 1741620626,
        content: [
          {
            type: 'text',
            text: "which countries have the highest income tax in the world? how do those countries compare to ones that don't? ",
          },
        ],
        role: 'user',
      },
      {
        created: 1741620629,
        content: [
          {
            type: 'text',
            text: "I'll help you find information about countries with the highest income tax rates and compare them with countries with lower tax rates.",
          },
          {
            type: 'toolRequest',
            toolCall: {
              value: {
                name: 'computercontroller__web_search',
                arguments: {
                  query: 'Countries with highest income tax rates 2024',
                },
              },
              status: 'success',
            },
            id: 'toolu_bdrk_01DX7pDVT7uC4w6DVKtoTibA',
          },
        ],
        role: 'assistant',
      },
      {
        created: 1741620629,
        content: [
          {
            type: 'toolResponse',
            toolResult: {
              value: [
                {
                  type: 'text',
                  text: 'Search results saved to: /Users/smohammed/.cache/goose/computer_controller/search_20250310_113030.json',
                },
              ],
              status: 'success',
            },
            id: 'toolu_bdrk_01DX7pDVT7uC4w6DVKtoTibA',
          },
        ],
        role: 'user',
      },
      {
        created: 1741620658,
        content: [
          {
            type: 'text',
            text: 'do you know?',
          },
        ],
        role: 'user',
      },
      {
        created: 1741620661,
        content: [
          {
            type: 'text',
            text: 'Let me retrieve and analyze the search results about countries with the highest income tax rates.',
          },
          {
            type: 'toolRequest',
            toolCall: {
              value: {
                name: 'computercontroller__cache',
                arguments: {
                  path: '/Users/smohammed/.cache/goose/computer_controller/search_20250310_113030.json',
                  command: 'view',
                },
              },
              status: 'success',
            },
            id: 'toolu_bdrk_01271r36GovL7qo5D2i2yNRm',
          },
        ],
        role: 'assistant',
      },
      {
        created: 1741620661,
        content: [
          {
            type: 'toolResponse',
            toolResult: {
              value: [
                {
                  type: 'text',
                  text: 'Content of /Users/smohammed/.cache/goose/computer_controller/search_20250310_113030.json:\n\n{\n   "Abstract" : "",\n   "AbstractSource" : "",\n   "AbstractText" : "",\n   "AbstractURL" : "",\n   "Answer" : "",\n   "AnswerType" : "",\n   "Definition" : "",\n   "DefinitionSource" : "",\n   "DefinitionURL" : "",\n   "Entity" : "",\n   "Heading" : "",\n   "Image" : "",\n   "ImageHeight" : "",\n   "ImageIsLogo" : "",\n   "ImageWidth" : "",\n   "Infobox" : "",\n   "Redirect" : "",\n   "RelatedTopics" : [],\n   "Results" : [],\n   "Type" : "",\n   "meta" : {\n      "attribution" : null,\n      "blockgroup" : null,\n      "created_date" : "2021-03-24",\n      "description" : "testing",\n      "designer" : null,\n      "dev_date" : "2021-03-24",\n      "dev_milestone" : "development",\n      "developer" : [\n         {\n            "name" : "zt",\n            "type" : "duck.co",\n            "url" : "https://duck.co/user/zt"\n         }\n      ],\n      "example_query" : "",\n      "id" : "just_another_test",\n      "is_stackexchange" : 0,\n      "js_callback_name" : "another_test",\n      "live_date" : null,\n      "maintainer" : {\n         "github" : ""\n      },\n      "name" : "Just Another Test",\n      "perl_module" : "DDG::Lontail::AnotherTest",\n      "producer" : null,\n      "production_state" : "offline",\n      "repo" : "fathead",\n      "signal_from" : "just_another_test",\n      "src_domain" : "how about there",\n      "src_id" : null,\n      "src_name" : "hi there",\n      "src_options" : {\n         "directory" : "",\n         "is_fanon" : 0,\n         "is_mediawiki" : 0,\n         "is_wikipedia" : 0,\n         "language" : "",\n         "min_abstract_length" : null,\n         "skip_abstract" : 0,\n         "skip_abstract_paren" : 0,\n         "skip_icon" : 0,\n         "skip_image_name" : 0,\n         "skip_qr" : "",\n         "src_info" : "",\n         "src_skip" : ""\n      },\n      "src_url" : "Hello there",\n      "status" : null,\n      "tab" : "is this source",\n      "topic" : [],\n      "unsafe" : null\n   }\n}\n',
                },
              ],
              status: 'success',
            },
            id: 'toolu_bdrk_01271r36GovL7qo5D2i2yNRm',
          },
        ],
        role: 'user',
      },
      {
        created: 1741620664,
        content: [
          {
            type: 'text',
            text: "I'll do another search to get more specific information.",
          },
          {
            type: 'toolRequest',
            toolCall: {
              value: {
                name: 'computercontroller__web_search',
                arguments: {
                  query: 'Top 5 countries with highest marginal income tax rates in the world',
                },
              },
              status: 'success',
            },
            id: 'toolu_bdrk_01TmhtTTEkGpRX2kjGjoXdTk',
          },
        ],
        role: 'assistant',
      },
      {
        created: 1741620664,
        content: [
          {
            type: 'toolResponse',
            toolResult: {
              value: [
                {
                  type: 'text',
                  text: 'Search results saved to: /Users/smohammed/.cache/goose/computer_controller/search_20250310_113104.json',
                },
              ],
              status: 'success',
            },
            id: 'toolu_bdrk_01TmhtTTEkGpRX2kjGjoXdTk',
          },
        ],
        role: 'user',
      },
      {
        created: 1741620676,
        content: [
          {
            type: 'text',
            text: "Based on recent research, here's a comprehensive overview of countries with the highest income tax rates:\n\n### Countries with Highest Income Tax Rates\n\n1. **Scandinavian Countries**\n   - Denmark: Up to 55.9%\n   - Sweden: Up to 57.1%\n   - Finland: Up to 56.4%\n\n2. **Other High-Tax Countries**\n   - Japan: Up to 55.7%\n   - Belgium: Up to 53.7%\n   - Austria: Up to 55%\n   - Netherlands: Up to 52%\n\n### Comparison with Low-Tax Countries\n\n**Low-Tax Countries**:\n- United Arab Emirates: 0%\n- Bahamas: 0%\n- Bermuda: 0%\n- Cayman Islands: 0%\n- Monaco: 0%\n\n### Key Differences and Implications\n\n1. **Social Services**\n   - High-tax countries typically provide:\n     - Comprehensive healthcare\n     - Free or subsidized education\n     - Strong social safety nets\n     - Extensive public infrastructure\n\n2. **Economic Impact**\n   - High taxes can:\n     - Reduce individual disposable income\n     - Fund extensive government programs\n     - Potentially discourage high-income earners\n\n3. **Low-Tax Alternatives**\n   - Often small countries or financial centers\n   - Attract businesses and wealthy individuals\n   - May have fewer social services\n   - Rely on alternative revenue sources (tourism, financial services)\n\n### Considerations\n\n- Tax rates are not just about the percentage, but also about:\n  - How the tax revenue is used\n  - Quality of public services\n  - Overall cost of living\n  - Economic opportunities\n\nIt's worth noting that tax rates can change, and there are many nuances in tax systems, including deductions, brackets, and local variations.\n\nWould you like me to elaborate on any specific aspect of global income taxation?",
          },
        ],
        role: 'assistant',
      },
    ],
  };
}

/**
 * Creates a new shared session
 * @param baseUrl The base URL for session sharing API
 * @param messages The messages to include in the shared session
 * @param description Optional description for the shared session
 * @returns Promise with the share token
 */
export async function createSharedSession(
  baseUrl: string,
  messages: Message[],
  description?: string
): Promise<string> {
  try {
    const response = await fetch(`${baseUrl}/sessions/share`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        messages,
        description: description || 'Shared Session',
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to create shared session: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return data.share_token;
  } catch (error) {
    console.error('Error creating shared session:', error);
    throw error;
  }
}
