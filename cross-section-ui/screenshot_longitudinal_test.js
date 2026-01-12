const puppeteer = require('puppeteer');
(async () => {
  const browser = await puppeteer.launch({ headless: 'new' });
  const page = await browser.newPage();
  await page.setViewport({ width: 1400, height: 900 });
  await page.goto('http://localhost:9080', { waitUntil: 'networkidle0', timeout: 30000 });
  await new Promise(r => setTimeout(r, 2000));

  // 縦断ボタンをクリック
  await page.mouse.click(149, 115);
  await new Promise(r => setTimeout(r, 2000));

  await page.screenshot({ path: 'screenshot_longitudinal_final.png' });
  console.log('Screenshot saved');
  await browser.close();
})();
