const puppeteer = require('puppeteer');
(async () => {
    const browser = await puppeteer.launch({ headless: true });
    
    // Desktop
    let page = await browser.newPage();
    await page.setViewport({ width: 1200, height: 900 });
    await page.goto('http://localhost:9010/', { waitUntil: 'networkidle0', timeout: 30000 });
    await new Promise(r => setTimeout(r, 3000));
    await page.screenshot({ path: 'screenshot_desktop.png' });
    await page.close();
    
    // Mobile
    page = await browser.newPage();
    await page.setViewport({ width: 390, height: 844, isMobile: true });
    await page.goto('http://localhost:9010/', { waitUntil: 'networkidle0', timeout: 30000 });
    await new Promise(r => setTimeout(r, 3000));
    await page.screenshot({ path: 'screenshot_mobile_new.png' });
    
    console.log('Screenshots saved');
    await browser.close();
})();
