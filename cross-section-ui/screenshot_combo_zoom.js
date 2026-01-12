const puppeteer = require('puppeteer');
(async () => {
  const browser = await puppeteer.launch({ headless: 'new' });
  const page = await browser.newPage();
  await page.setViewport({ width: 1400, height: 900 });
  await page.goto('http://localhost:9085', { waitUntil: 'networkidle0', timeout: 30000 });
  await new Promise(r => setTimeout(r, 3000));
  // 横断図部分を拡大
  await page.screenshot({ path: 'screenshot_combo_zoom.png', clip: { x: 270, y: 230, width: 200, height: 120 } });
  console.log('Screenshot saved');
  await browser.close();
})();
