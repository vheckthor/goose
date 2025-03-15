import { _electron as electron } from 'playwright';

export const startApp = async () =>
  await electron.launch({
    args: ['out/Goose-darwin-arm64/Goose.app/Contents/Resources/app.asar/.vite/build/main.js'],
    executablePath: 'out/Goose-darwin-arm64/Goose.app/Contents/MacOS/Goose',
    recordVideo: { dir: 'test-results/videos/' },
    offline: true,
  });
