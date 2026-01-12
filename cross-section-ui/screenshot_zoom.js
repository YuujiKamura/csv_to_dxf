const puppeteer = require('puppeteer');
(async () => {
  const browser = await puppeteer.launch({ headless: 'new' });
  const page = await browser.newPage();
  await page.setViewport({ width: 1400, height: 900 });
  await page.goto('http://localhost:9084', { waitUntil: 'networkidle0', timeout: 30000 });
  await new Promise(r => setTimeout(r, 2000));
  await page.mouse.click(149, 115);
  await new Promise(r => setTimeout(r, 2000));
  // テーブル最初の列を拡大
  await page.screenshot({ path: 'screenshot_zoom.png', clip: { x: 380, y: 530, width: 80, height: 80 } });
  console.log('Screenshot saved');
  await browser.close();
})();
