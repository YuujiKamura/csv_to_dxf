const puppeteer = require('puppeteer');

(async () => {
    const browser = await puppeteer.launch({
        headless: true,
        args: ['--no-sandbox', '--disable-setuid-sandbox']
    });
    const page = await browser.newPage();
    await page.setViewport({ width: 1280, height: 900 });

    console.log('Loading page...');
    await page.goto('http://localhost:9011/', { waitUntil: 'networkidle0', timeout: 30000 });

    // Wait for canvas and WASM to load
    await page.waitForSelector('canvas', { timeout: 10000 });
    await new Promise(r => setTimeout(r, 3000));

    await page.screenshot({ path: 'screenshot_release.png', fullPage: false });
    console.log('Screenshot saved: screenshot_release.png');

    // Check for any error overlays
    const hasOverlay = await page.evaluate(() => {
        const overlay = document.querySelector('div[style*="position: fixed"]');
        return overlay ? overlay.innerText : null;
    });

    if (hasOverlay) {
        console.log('Overlay detected:', hasOverlay);
    } else {
        console.log('No overlay detected');
    }

    await browser.close();
})();
