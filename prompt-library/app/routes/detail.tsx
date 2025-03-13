import { useParams, Link } from "react-router";
import {
  Copy,
  Star,
  Tag,
  ArrowLeft,
  Info,
  Calendar,
  User,
  Wrench
} from "lucide-react";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { Card, CardContent, CardHeader } from "../components/ui/card";
import { useEffect, useState } from "react";
import { fetchPrompts } from "../prompt-data";
import type { Prompt } from "../types/prompt";

export default function DetailPage() {
  const { id } = useParams();
  const [prompt, setPrompt] = useState<Prompt | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const loadPrompt = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const prompts = await fetchPrompts();
        const foundPrompt = prompts.find((p) => p.id === id);
        if (!foundPrompt) {
          setError(`Prompt with ID "${id}" not found`);
          return;
        }
        setPrompt(foundPrompt);
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : "Unknown error";
        setError(`Failed to load prompt: ${errorMessage}`);
        console.error("Error loading prompt:", err);
      } finally {
        setIsLoading(false);
      }
    };

    loadPrompt();
  }, [id]);

  if (!prompt) {
    return (
      <div className="max-w-4xl mx-auto">
        <div className="flex items-center gap-2 mb-6">
          <Link to="/">
            <Button className="">
              <ArrowLeft className="h-4 w-4" />
              Back
            </Button>
          </Link>
          <div className="text-sm text-gray-500 dark:text-gray-400">
            <Link
              to="/"
              className="hover:text-accent dark:hover:text-accent/90"
            >
              Goose Prompts
            </Link>{" "}
            /
          </div>
        </div>
        <div className="animate-pulse">
          <div className="h-8 w-48 bg-gray-200 dark:bg-gray-700 rounded mb-4"></div>
          <div className="h-4 w-full bg-gray-200 dark:bg-gray-700 rounded mb-2"></div>
          <div className="h-4 w-2/3 bg-gray-200 dark:bg-gray-700 rounded"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto">
      <div className="flex gap-8 max-w-5xl mx-auto">
        <div>
          <Link to="/">
            <Button className="flex items-center gap-2">
              <ArrowLeft className="h-4 w-4" />
              Back
            </Button>
          </Link>
        </div>

        <Card className="p-8 w-full">
          <CardHeader className="flex items-center">
            <div className="flex items-center gap-2">
              <h1 className="font-medium text-5xl text-textProminent detail-page-prompt-title">
                {prompt.title}
              </h1>
            </div>
          </CardHeader>
          <CardContent className="space-y-6">
            <div>
              <p className="text-xl text-textSubtle">{prompt.description}</p>
            </div>

            <div className="space-y-2">
              <div className="flex items-center gap-2 text-textStandard">
                <Tag className="h-4 w-4" />
                <h4 className="font-medium">Category & Role</h4>
              </div>
              <div className="flex gap-2">
                <Badge variant="secondary">{prompt.category}</Badge>
                <Badge variant="outline">{prompt.role}</Badge>
              </div>
            </div>

            <div className="space-y-2">
              <div className="flex items-center gap-2 text-textStandard">
                <Info className="h-4 w-4" />
                <h4 className="font-medium">Prompt Template</h4>
              </div>
              <code className="block bg-gray-100 dark:bg-gray-900 p-4 rounded text-sm dark:text-gray-300 whitespace-pre-wrap">
                {prompt.prompt}
              </code>
              <Button 
                variant="outline" 
                size="sm"
                className="mt-2"
                onClick={() => navigator.clipboard.writeText(prompt.prompt)}
              >
                <Copy className="h-4 w-4 mr-2" />
                Copy to Clipboard
              </Button>
            </div>

            {prompt.example && (
              <div className="space-y-2">
                <div className="flex items-center gap-2 text-textStandard">
                  <Info className="h-4 w-4" />
                  <h4 className="font-medium">Example Usage</h4>
                </div>
                <div className="bg-gray-50 dark:bg-gray-900 p-4 rounded text-sm">
                  {prompt.example}
                </div>
              </div>
            )}

            {prompt.extensions && prompt.extensions.length > 0 && (
              <div className="space-y-2">
                <div className="flex items-center gap-2 text-textStandard">
                  <Wrench className="h-4 w-4" />
                  <h4 className="font-medium">Required Extensions</h4>
                </div>
                <div className="flex gap-2 flex-wrap">
                  {prompt.extensions.map(extension => (
                    <Badge key={extension} variant="outline">{extension}</Badge>
                  ))}
                </div>
              </div>
            )}

            {prompt.variables && prompt.variables.length > 0 && (
              <div className="space-y-4">
                <h2 className="text-lg font-medium dark:text-gray-300">
                  Variables
                </h2>
                <div className="">
                  {prompt.variables.map((variable) => (
                    <div
                      key={variable.name}
                      className="border-b border-borderSubtle pb-4 mb-4 last:border-0"
                    >
                      <div className="text-sm dark:text-gray-300">
                        {variable.name}
                      </div>
                      <div className="text-gray-600 dark:text-gray-400 text-sm mt-1">
                        {variable.description}
                      </div>
                      <div className="flex gap-2 mt-2">
                        {variable.required && (
                          <Badge variant="secondary">Required</Badge>
                        )}
                        <Badge variant="outline">{variable.type}</Badge>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            <div className="flex items-center justify-between border-t border-borderSubtle pt-4">
              <div className="flex items-center gap-4 text-sm text-gray-500 dark:text-gray-400">
                <div className="flex items-center gap-2">
                  <User className="h-4 w-4" />
                  <span>{prompt.author}</span>
                </div>
                <div className="flex items-center gap-2">
                  <Calendar className="h-4 w-4" />
                  <span>Updated: {new Date(prompt.lastUpdated).toLocaleDateString()}</span>
                </div>
              </div>

              <div className="flex items-center gap-2">
                <div className="flex items-center gap-1">
                  <Star className="h-4 w-4 text-yellow-400" />
                  <span className="text-sm">{prompt.rating.toFixed(1)}</span>
                </div>
                {prompt.verified && (
                  <Badge variant="secondary" className="ml-2">
                    Verified
                  </Badge>
                )}
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}