import {chromium} from 'playwright';
import assert from 'node:assert/strict';
const browser=await chromium.launch({headless:true,executablePath:process.env.HYPER_BROWSER||undefined,args:['--enable-unsafe-webgpu',...(process.platform==='darwin'?['--use-angle=metal']:[])]});
try {
  const page=await browser.newPage({viewport:{width:1440,height:1050},deviceScaleFactor:1});
  await page.goto(process.env.HYPER_DEMO_URL??'http://127.0.0.1:8080/demo/');
  await page.waitForFunction(()=>[...document.querySelectorAll('.backend')].length===2&&[...document.querySelectorAll('.backend')].every(element=>['webgpu','webgl2'].includes(element.textContent)));
  const viewers=page.locator('.viewer');
  await viewers.nth(0).locator('.pause').click();await page.waitForFunction(()=>document.querySelector('.pause').textContent==='Resume layout');
  await viewers.nth(0).locator('.lifecycle').click();
  assert.equal(await viewers.nth(0).locator('.frame').isDisabled(),true);
  assert.equal(await viewers.nth(1).locator('.frame').isEnabled(),true);
  await viewers.nth(0).locator('.lifecycle').click();
  await page.waitForFunction(()=>document.querySelector('.lifecycle').textContent==='Dispose');
  assert.equal(await page.locator('.error:visible').count(),0);
  assert.match(await viewers.nth(0).locator('.status').textContent(),/5 vertices/);
  await page.waitForFunction(()=>[...document.querySelectorAll('.status')].every(element=>/layout steps/.test(element.textContent)));
  await page.waitForTimeout(500);
  await page.screenshot({path:process.env.HYPER_DEMO_SCREENSHOT??'/tmp/hyper-browser-demo.png',fullPage:true});
  console.log('PASS two-view demo loads, pauses, disposes and remounts independently');
} finally {await browser.close();}
