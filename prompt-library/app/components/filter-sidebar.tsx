import { Package, Puzzle } from "lucide-react";
import { Checkbox } from "./ui/checkbox";
import { Label } from "./ui/label";
import { cn } from "../lib/utils";

interface FilterSidebarProps {
  filters: {
    roles: string[];
    extensions: string[];
  };
  selected: {
    roles: string[];
    extensions: string[];
  };
  onFilterChange: (type: string, value: string) => void;
}

export function FilterSidebar({ 
  filters, 
  selected, 
  onFilterChange,
}: FilterSidebarProps) {
  // Define the available extensions
  const extensions = [
    "developer",
    "computer controller",
    "memory",
    "jetbrains",
    "git",
    "figma",
    "google drive"
  ];

  return (
    <div className="w-64 pr-6 space-y-6">
      {/* Role Filter */}
      <div className="space-y-3">
        <h3 className="flex items-center gap-2 text-sm font-medium text-textProminent">
          <Package className="h-4 w-4" />
          Roles
        </h3>
        <div className="space-y-2">
          {filters.roles.map((role) => (
            <div key={role} className="flex items-center space-x-2">
              <Checkbox 
                id={`role-${role}`}
                checked={selected.roles.includes(role)}
                onCheckedChange={() => onFilterChange('roles', role)}
                className="border-borderSubtle"
              />
              <Label htmlFor={`role-${role}`} className="text-sm text-textStandard">
                {role}
              </Label>
            </div>
          ))}
        </div>
      </div>

      {/* Extensions Filter */}
      <div className="space-y-3">
        <h3 className="flex items-center gap-2 text-sm font-medium text-textProminent">
          <Puzzle className="h-4 w-4" />
          Extensions
        </h3>
        <div className="space-y-2">
          {extensions.map((extension) => (
            <div key={extension} className="flex items-center space-x-2">
              <Checkbox 
                id={`extension-${extension}`}
                checked={selected.extensions.includes(extension)}
                onCheckedChange={() => onFilterChange('extensions', extension)}
                className="border-borderSubtle"
              />
              <Label htmlFor={`extension-${extension}`} className="text-sm text-textStandard">
                {extension}
              </Label>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}