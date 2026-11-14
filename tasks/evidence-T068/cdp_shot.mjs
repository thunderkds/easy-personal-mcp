// node cdp_shot.mjs URL WIDTH HEIGHT OUT "<js condition>" -> waits until the condition is true, then screenshots; prints measurements
import { spawn } from 'node:child_process'
import { writeFileSync } from 'node:fs'
const [url, w, h, out, cond = 'true'] = process.argv.slice(2)
const port = 9300 + Math.floor(Math.random() * 500)
const chrome = spawn('google-chrome', ['--headless=new', '--no-sandbox', '--disable-gpu', '--hide-scrollbars', `--remote-debugging-port=${port}`, `--user-data-dir=/tmp/cdp-${port}`, 'about:blank'], { stdio: 'ignore' })
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
let targets
for (let i = 0; i < 50; i++) { try { targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json(); if (targets.find((t) => t.type === 'page')) break } catch {} await sleep(200) }
const ws = new WebSocket(targets.find((t) => t.type === 'page').webSocketDebuggerUrl)
await new Promise((r) => (ws.onopen = r))
let id = 0; const pending = new Map()
ws.onmessage = (m) => { const d = JSON.parse(m.data); if (d.id && pending.has(d.id)) { pending.get(d.id)(d); pending.delete(d.id) } }
const send = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })) })
const evaluate = async (expression) => (await send('Runtime.evaluate', { expression, returnByValue: true })).result?.result?.value
await send('Emulation.setDeviceMetricsOverride', { width: +w, height: +h, deviceScaleFactor: 1, mobile: false })
await send('Page.navigate', { url })
let ok = false
for (let i = 0; i < 100 && !ok; i++) { await sleep(200); ok = await evaluate(cond) }
await sleep(300)
const info = await evaluate(`JSON.stringify({ok:${'true'}, scrollW: document.documentElement.scrollWidth, innerW: innerWidth, chips: [...document.querySelectorAll('.status-chip')].map(c=>{const r=c.getBoundingClientRect();const s=getComputedStyle(c);return {t:c.textContent,top:Math.round(r.top),left:Math.round(r.left),w:Math.round(r.width),h:Math.round(r.height),font:s.fontFamily.slice(0,20)+' '+s.fontSize,gap:s.columnGap,pad:s.padding,radius:s.borderRadius,color:s.color,cls:c.className}})})`)
const shot = await send('Page.captureScreenshot', { format: 'png' })
writeFileSync(out, Buffer.from(shot.result.data, 'base64'))
console.log('cond met:', ok); console.log(info)
ws.close(); chrome.kill()
process.exit(0)
