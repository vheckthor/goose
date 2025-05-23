import type { MCPServer } from "../types/server";

const API_BASE_URL = 'http://localhost:3001';

interface APIPackage {
  registry_name: string;
  name: string;
  version: string;
  runtime_hint: string;
  runtime_arguments: {
    is_required: boolean;
    format: string;
    value: string;
    default: string;
    type: string;
    value_hint: string;
  }[];
  environment_variables: {
    name: string;
    description: string;
  }[];
}

interface APIListServer {
  id: string;
  name: string;
  description: string;
  repository: {
    url: string;
  };
}

interface APIDetailedServer {
  id: string;
  name: string;
  description: string;
  repository: {
    url: string;
    source: string;
    id: string;
  };
  version_detail: {
    version: string;
    release_date: string;
    is_latest: boolean;
  };
  package_canonical: string;
  packages: APIPackage[];
}

interface APIResponse {
  servers: APIListServer[];
  metadata: {
    next_cursor: string;
    count: number;
  };
}

function transformListServer(apiServer: APIListServer): MCPServer {
  return {
    id: apiServer.id,
    name: apiServer.name,
    description: apiServer.description,
    url: apiServer.repository.url,
    command: "", // Will be populated in detail view
    link: apiServer.repository.url,
    installation_notes: "",
    is_builtin: false,
    endorsed: false,
    githubStars: 0,
    environmentVariables: []
  };
}

function transformDetailedServer(apiServer: APIDetailedServer): MCPServer {
  // Get the canonical package (first one for now)
  const pkg = apiServer.packages[0];
  
  // For command-based servers (those with runtime_hint), construct the command
  // For hosted servers, leave command empty
  const isCommandBased = pkg.runtime_hint && pkg.runtime_arguments.length > 0;
  const command = isCommandBased ? `${pkg.runtime_hint} ${pkg.runtime_arguments.map(arg => arg.value).join(" ")}` : "";

  // Transform environment variables
  const envVars = (pkg.environment_variables || []).map(env => ({
    name: env.name,
    description: env.description,
    required: true // Assuming all are required for now
  }));
  
  return {
    id: apiServer.id,
    name: apiServer.name,
    description: apiServer.description,
    url: isCommandBased ? "" : apiServer.repository.url, // Only set URL for hosted servers
    command: command, // Only set command for command-based servers
    link: apiServer.repository.url,
    installation_notes: "",
    is_builtin: false,
    endorsed: false,
    githubStars: 0,
    environmentVariables: envVars
  };
}

export async function fetchMCPServers(): Promise<MCPServer[]> {
  try {
    const response = await fetch(`${API_BASE_URL}/api/v0/servers?limit=1000`);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const data: APIResponse = await response.json();
    console.log('Fetched MCP servers data:', data);
    
    return data.servers.map(transformListServer);
  } catch (error) {
    console.error("Error fetching MCP servers:", error);
    throw error;
  }
}

export async function fetchServerDetails(id: string): Promise<MCPServer> {
  try {
    const response = await fetch(`${API_BASE_URL}/api/v0/servers/${id}`);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const data: APIDetailedServer = await response.json();
    console.log('Fetched server details:', data);
    
    return transformDetailedServer(data);
  } catch (error) {
    console.error("Error fetching server details:", error);
    throw error;
  }
}

export async function searchMCPServers(query: string): Promise<MCPServer[]> {
  const servers = await fetchMCPServers();
  const normalizedQuery = query.toLowerCase();
  
  return servers.filter((server) => {
    const normalizedName = server.name.toLowerCase();
    const normalizedDescription = server.description.toLowerCase();
    
    return (
      normalizedName.includes(normalizedQuery) ||
      normalizedDescription.includes(normalizedQuery)
    );
  });
}