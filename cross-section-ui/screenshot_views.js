const puppeteer = require('puppeteer');
(async () => {
    const browser = await puppeteer.launch({ headless: true });
    const page = await browser.newPage();
    await page.setViewport({ width: 1400, height: 900 });
    await page.goto('http://localhost:9030/index_static.html', { waitUntil: 'networkidle0', timeout: 30000 });
    await new Promise(r => setTimeout(r, 2000));

    // Screenshot 1: Initial single view (単一)
    await page.screenshot({ path: 'screenshot_view_single.png', fullPage: false });
    console.log('Screenshot saved: screenshot_view_single.png');

    // The buttons appear around: 表示: [単一] [全横断] [縦断]
    // Looking at the screenshot, buttons are at approximately:
    // 単一 (selected): x=57-75, y=63
    // 全横断: x=80-110, y=63
    // 縦断: x=115-145, y=63

    // Screenshot 2: Click on 全横断 (second button, around x=95)
    await page.mouse.click(95, 63);
    await new Promise(r => setTimeout(r, 2000));
    await page.screenshot({ path: 'screenshot_view_allgrid.png', fullPage: false });
    console.log('Screenshot saved: screenshot_view_allgrid.png');

    // Screenshot 3: Click on 縦断 (third button, around x=150)
    await page.mouse.click(150, 63);
    await new Promise(r => setTimeout(r, 2000));
    await page.screenshot({ path: 'screenshot_view_longitudinal.png', fullPage: false });
    console.log('Screenshot saved: screenshot_view_longitudinal.png');

    await browser.close();
})();
