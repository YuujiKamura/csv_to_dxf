const puppeteer = require('puppeteer');
(async () => {
  const browser = await puppeteer.launch({ headless: 'new' });
  const page = await browser.newPage();
  await page.setViewport({ width: 1400, height: 900 });
  await page.goto('http://localhost:9082', { waitUntil: 'networkidle0', timeout: 30000 });
  await new Promise(r => setTimeout(r, 2000));
  await page.mouse.click(149, 115);
  await new Promise(r => setTimeout(r, 2000));
  // テーブル部分を拡大
  await page.screenshot({ path: 'screenshot_table_detail.png', clip: { x: 350, y: 520, width: 300, height: 120 } });
  console.log('Screenshot saved');
  await browser.close();
})();
