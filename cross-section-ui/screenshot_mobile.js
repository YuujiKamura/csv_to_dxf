const puppeteer = require('puppeteer');
(async () => {
    const browser = await puppeteer.launch({ headless: true });
    const page = await browser.newPage();
    await page.setViewport({ width: 390, height: 844, isMobile: true });
    await page.goto('https://yuujikamura.github.io/csv_to_dxf/', { waitUntil: 'networkidle0', timeout: 60000 });
    await new Promise(r => setTimeout(r, 5000));
    await page.screenshot({ path: 'screenshot_mobile.png', fullPage: false });
    console.log('Screenshot saved');
    await browser.close();
})();
