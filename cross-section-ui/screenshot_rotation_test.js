const puppeteer = require('puppeteer');
(async () => {
  const browser = await puppeteer.launch({ headless: 'new' });
  const page = await browser.newPage();

  // コンソールログをキャプチャ
  const consoleLogs = [];
  page.on('console', msg => {
    if (msg.text().includes('rotation')) {
      consoleLogs.push(msg.text());
    }
  });

  await page.setViewport({ width: 1400, height: 900 });
  await page.goto('http://localhost:9080', { waitUntil: 'networkidle0', timeout: 30000 });
  await new Promise(r => setTimeout(r, 2000));

  // 縦断ボタンをクリック
  await page.evaluate(() => {
    const canvas = document.querySelector('canvas');
    if (canvas) {
      // 縦断ボタンの位置をクリック (右上付近)
      const rect = canvas.getBoundingClientRect();
      const event = new MouseEvent('click', {
        clientX: rect.right - 100,
        clientY: rect.top + 30,
        bubbles: true
      });
      canvas.dispatchEvent(event);
    }
  });
  await new Promise(r => setTimeout(r, 2000));

  // コンソールログを出力
  console.log('=== Console Logs ===');
  consoleLogs.forEach(log => console.log(log));
  console.log('=== Total rotation logs:', consoleLogs.length, '===');

  await page.screenshot({ path: 'screenshot_rotation_debug.png' });
  console.log('Screenshot saved to screenshot_rotation_debug.png');

  await browser.close();
})();
