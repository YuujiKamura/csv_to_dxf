const puppeteer = require('puppeteer');
(async () => {
    const browser = await puppeteer.launch({ headless: true });
    let page = await browser.newPage();
    await page.setViewport({ width: 1400, height: 900 });
    await page.goto('http://localhost:9010/', { waitUntil: 'networkidle0', timeout: 60000 });
    // Wait longer for WASM to load
    await new Promise(r => setTimeout(r, 8000));
    await page.screenshot({ path: 'screenshot_single.png' });
    console.log('Screenshot saved');
    await browser.close();
})();
