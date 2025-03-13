import { Input } from "../components/ui/input";
import { PromptCard } from "../components/prompt-card";
import { FilterPills } from "../components/filter-pills";
import { FilterSidebar } from "../components/filter-sidebar";
import { useState, useEffect } from "react";
import type { Prompt } from "../types/prompt";
import { fetchPrompts, searchPrompts } from "../prompt-data";
import { motion } from "framer-motion";

export default function HomePage() {
  const [prompts, setPrompts] = useState<Prompt[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [filters, setFilters] = useState({
    roles: [] as string[],
    tools: [] as string[],
    tags: [] as string[],
    verified: false
  });
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Get unique categories, roles, tools, and tags from prompts
  const categories = [...new Set(prompts.map(p => p.category))];
  const allFilters = {
    roles: [...new Set(prompts.map(p => p.role))],
    tools: [...new Set(prompts.flatMap(p => p.tools))],
    tags: [...new Set(prompts.flatMap(p => p.tags))]
  };

  useEffect(() => {
    const loadPrompts = async () => {
      try {
        setIsLoading(true);
        setError(null);

        const trimmedQuery = searchQuery.trim();
        const results = trimmedQuery
          ? await searchPrompts(trimmedQuery)
          : await fetchPrompts();

        setPrompts(results);
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : "Unknown error";
        setError(`Failed to load prompts: ${errorMessage}`);
        console.error("Error loading prompts:", err);
      } finally {
        setIsLoading(false);
      }
    };

    const timeoutId = setTimeout(loadPrompts, 300);
    return () => clearTimeout(timeoutId);
  }, [searchQuery]);

  // Filter prompts based on all criteria
  const filteredPrompts = prompts.filter(prompt => {
    const matchesCategory = !selectedCategory || prompt.category === selectedCategory;
    const matchesRoles = filters.roles.length === 0 || filters.roles.includes(prompt.role);
    const matchesTools = filters.tools.length === 0 || prompt.tools.some(t => filters.tools.includes(t));
    const matchesTags = filters.tags.length === 0 || prompt.tags.some(t => filters.tags.includes(t));
    const matchesVerified = !filters.verified || prompt.verified;
    const matchesSearch = !searchQuery || 
      prompt.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      prompt.description.toLowerCase().includes(searchQuery.toLowerCase());

    return matchesCategory && matchesRoles && matchesTools && matchesTags && matchesVerified && matchesSearch;
  });

  // Handle filter changes
  const handleFilterChange = (type: keyof typeof filters, value: string) => {
    setFilters(prev => ({
      ...prev,
      [type]: prev[type].includes(value) 
        ? prev[type].filter(v => v !== value)
        : [...prev[type], value]
    }));
  };

  return (
    <div className="pb-24">
      <div className="pb-16">
        <h1 className="text-[64px] font-medium text-textProminent">
          Prompt Library
        </h1>
        <p className="text-textProminent">
          Your central directory for discovering and using effective prompts with Goose.
        </p>
      </div>

      <div className="relative mb-6">
        <Input
          className="pl-0"
          placeholder="Search prompts by category, role, or keyword"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />
      </div>

      <div className="mb-8">
        <FilterPills
          categories={categories}
          selectedCategory={selectedCategory}
          onSelectCategory={setSelectedCategory}
        />
      </div>

      <div className="flex gap-8">
        <FilterSidebar
          filters={allFilters}
          selected={filters}
          onFilterChange={handleFilterChange}
          onVerifiedChange={(checked) => setFilters(prev => ({ ...prev, verified: checked }))}
        />

        <div className="flex-1">
          {error && (
            <div className="p-4 bg-red-50 text-red-600 rounded-md">{error}</div>
          )}


          {isLoading ? (
            <div className="py-8 text-xl text-textSubtle">Loading prompts...</div>
          ) : filteredPrompts.length === 0 ? (
            <div className="text-center py-8 text-gray-500">
              {searchQuery
                ? "No prompts found matching your search."
                : "No prompts available."}
            </div>
          ) : (
            <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
              {filteredPrompts.map((prompt) => (
                <motion.div
                  key={prompt.id}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.6 }}
                >
                  <PromptCard prompt={prompt} />
                </motion.div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}