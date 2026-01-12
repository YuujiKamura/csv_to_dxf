const puppeteer = require('puppeteer');
(async () => {
    const browser = await puppeteer.launch({ headless: true });
    const page = await browser.newPage();
    await page.setViewport({ width: 1400, height: 900 });
    await page.goto('http://localhost:9020/', { waitUntil: 'networkidle0', timeout: 30000 });
    await new Promise(r => setTimeout(r, 2000));

    // First take screenshot of initial state
    await page.screenshot({ path: 'screenshot_initial.png', fullPage: false });
    console.log('Screenshot saved: screenshot_initial.png');

    // Click on "縦断" button - it should be around y=63 in the side panel
    // The buttons are: 表示: [単一] [全横断] [縦断]
    // Approximate x positions: 単一~110, 全横断~160, 縦断~220
    await page.mouse.click(220, 63);

    await new Promise(r => setTimeout(r, 2000));
    await page.screenshot({ path: 'screenshot_longitudinal.png', fullPage: false });
    console.log('Screenshot saved: screenshot_longitudinal.png');
    await browser.close();
})();
