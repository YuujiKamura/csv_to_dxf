const puppeteer = require('puppeteer');
(async () => {
    const browser = await puppeteer.launch({ headless: true });
    const page = await browser.newPage();
    await page.setViewport({ width: 1400, height: 900 });
    await page.goto('http://localhost:9010/', { waitUntil: 'networkidle0', timeout: 30000 });
    await new Promise(r => setTimeout(r, 2000));

    // egui renders to canvas - click by coordinates
    // "All" button is below "Load Sample" in the left panel
    // y=63 is approximate position of "All" button
    await page.mouse.click(50, 63);

    await new Promise(r => setTimeout(r, 2000));
    await page.screenshot({ path: 'screenshot_all_sections.png', fullPage: false });
    console.log('Screenshot saved');
    await browser.close();
})();
