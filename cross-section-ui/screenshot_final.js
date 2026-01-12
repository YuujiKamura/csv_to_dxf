const puppeteer = require('puppeteer');
(async () => {
    const browser = await puppeteer.launch({ headless: true });
    let page = await browser.newPage();
    await page.setViewport({ width: 1400, height: 900 });
    await page.goto('https://yuujikamura.github.io/csv_to_dxf/', { waitUntil: 'networkidle0', timeout: 60000 });
    await new Promise(r => setTimeout(r, 5000));
    await page.screenshot({ path: 'screenshot_final.png' });
    console.log('Screenshot saved');
    await browser.close();
})();
