const puppeteer = require('puppeteer');
(async () => {
    const browser = await puppeteer.launch({ headless: true });
    let page = await browser.newPage();
    
    // Capture console logs
    page.on('console', msg => console.log('PAGE LOG:', msg.text()));
    page.on('pageerror', err => console.log('PAGE ERROR:', err.message));
    
    await page.setViewport({ width: 1400, height: 900 });
    
    try {
        await page.goto('http://localhost:9010/', { waitUntil: 'networkidle0', timeout: 30000 });
    } catch(e) {
        console.log('Navigation error:', e.message);
    }
    
    await new Promise(r => setTimeout(r, 5000));
    
    // Check if canvas exists
    const canvasExists = await page.evaluate(() => {
        const canvas = document.getElementById('canvas');
        return canvas ? { width: canvas.width, height: canvas.height } : null;
    });
    console.log('Canvas:', canvasExists);
    
    await page.screenshot({ path: 'screenshot_debug.png' });
    console.log('Screenshot saved');
    await browser.close();
})();
