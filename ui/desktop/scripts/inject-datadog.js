const fs = require('fs');
const path = require('path');

// Path to your renderer.tsx file
const rendererPath = path.join(__dirname, '../src/renderer.tsx');
const rendererContent = fs.readFileSync(rendererPath, 'utf8');

// Add Datadog imports and initialization code
const datadogCode = `import { datadogRum } from '@datadog/browser-rum';

const DATADOG_APPLICATION_ID = '139d4946-a1a5-4d5f-b017-2422e7774b9d';
const DATADOG_CLIENT_TOKEN = 'puba9048a40434f456895695b2d552b9c5c';
const DATADOG_ENV = 'dev';

// Initialize Datadog RUM
datadogRum.init({
    applicationId: DATADOG_APPLICATION_ID,
    clientToken: DATADOG_CLIENT_TOKEN,
    site: 'datadoghq.com',
    service: 'goose',
    env: DATADOG_ENV,
    sessionSampleRate: 100,
    sessionReplaySampleRate: 20,
    trackUserInteractions: true,
    trackResources: true,
    trackLongTasks: true,
    defaultPrivacyLevel: 'mask-user-input',
});\n\n`;

// Insert the Datadog code at the top of the file
const modifiedContent = datadogCode + rendererContent;
fs.writeFileSync(rendererPath, modifiedContent);
console.log('Successfully injected Datadog configuration');