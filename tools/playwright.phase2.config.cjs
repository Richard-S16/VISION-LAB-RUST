const path = require("node:path");

const projectRoot = path.resolve(__dirname, "..");

module.exports = {
  testDir: __dirname,
  timeout: 3 * 60 * 1000,
  workers: 1,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:5173",
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
    headless: true,
    launchOptions: {
      executablePath: path.join(
        process.env.LOCALAPPDATA,
        "ms-playwright",
        "chromium-1234",
        "chrome-win64",
        "chrome.exe",
      ),
    },
  },
  webServer: {
    command: "npm run dev",
    cwd: projectRoot,
    port: 5173,
    reuseExistingServer: false,
    timeout: 120 * 1000,
  },
};
