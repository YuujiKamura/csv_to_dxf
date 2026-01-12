const puppeteer = require('puppeteer');
(async () => {
    const browser = await puppeteer.launch({ headless: true });
    let page = await browser.newPage();
    page.on('console', msg => console.log('PAGE:', msg.text()));
    page.on('pageerror', err => console.log('ERROR:', err.message));
    await page.setViewport({ width: 1400, height: 900 });
    await page.goto('http://localhost:9010/', { waitUntil: 'networkidle0', timeout: 60000 });
    await new Promise(r => setTimeout(r, 5000));
    await page.screenshot({ path: 'screenshot_multi_view.png' });
    console.log('Screenshot saved');
    await browser.close();
})();
