const puppeteer = require('puppeteer');
(async () => {
    const browser = await puppeteer.launch({ headless: true });
    
    // Desktop
    let page = await browser.newPage();
    await page.setViewport({ width: 1200, height: 900 });
    await page.goto('https://yuujikamura.github.io/csv_to_dxf/', { waitUntil: 'networkidle0', timeout: 60000 });
    await new Promise(r => setTimeout(r, 3000));
    await page.screenshot({ path: 'screenshot_pages_desktop.png' });
    await page.close();
    
    // Mobile
    page = await browser.newPage();
    await page.setViewport({ width: 390, height: 844, isMobile: true });
    await page.goto('https://yuujikamura.github.io/csv_to_dxf/', { waitUntil: 'networkidle0', timeout: 60000 });
    await new Promise(r => setTimeout(r, 3000));
    await page.screenshot({ path: 'screenshot_pages_mobile.png' });
    
    console.log('Screenshots saved');
    await browser.close();
})();
