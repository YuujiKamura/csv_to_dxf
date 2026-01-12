const puppeteer = require('puppeteer');
(async () => {
    const browser = await puppeteer.launch({ headless: true });
    const page = await browser.newPage();
    await page.setViewport({ width: 1400, height: 900 });
    await page.goto('http://localhost:9050/index_static.html', { waitUntil: 'networkidle0', timeout: 30000 });
    await new Promise(r => setTimeout(r, 3000));

    // Screenshot 1: Initial single view
    await page.screenshot({ path: 'verify_single.png', fullPage: false });
    console.log('verify_single.png saved');

    // Click on 縦断 button (third button, approximately x=150)
    await page.mouse.click(150, 63);
    await new Promise(r => setTimeout(r, 2000));
    await page.screenshot({ path: 'verify_longitudinal.png', fullPage: false });
    console.log('verify_longitudinal.png saved');

    await browser.close();
})();
