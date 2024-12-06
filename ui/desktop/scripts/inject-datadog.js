const fs = require('fs');
const path = require('path');

// Path to your renderer.tsx file
const rendererPath = path.join(__dirname, 'ui/desktop/src/renderer.tsx');
const rendererContent = fs.readFileSync(rendererPath, 'utf8');

// Add Datadog imports and initialization
const datadogImport = `import { initDatadog } from './datadog-config';\n`;
const initCall = `\n// Initialize Datadog RUM\ninitDatadog();\n`;

// Insert the import at the top of the file
const withImport = datadogImport + rendererContent;

// Find the appropriate spot to insert initialization (after Router import)
const lines = withImport.split('\n');
let insertIndex = -1;
for (let i = 0; i < lines.length; i++) {
    if (lines[i].includes('import { BrowserRouter as Router }')) {
        insertIndex = i + 1;
        break;
    }
}

if (insertIndex !== -1) {
    lines.splice(insertIndex, 0, initCall);
    const modifiedContent = lines.join('\n');
    fs.writeFileSync(rendererPath, modifiedContent);
    console.log('Successfully injected Datadog configuration');
} else {
    console.error('Could not find appropriate location to inject Datadog configuration');
    process.exit(1);
}