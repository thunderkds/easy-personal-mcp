// T134: stand-ins for `@xterm/xterm` and `@xterm/addon-fit` (jsdom has no layout or canvas) and for the browser's WebSocket and ResizeObserver.
// A test drives them by hand: `Terminal.last.type('ls\r')` is a key press, `FakeSocket.last.receive(bytes)` is a frame from the server.
import { act } from '@testing-library/react'

export class Terminal {
  static instances = []
  static get last() {
    return Terminal.instances.at(-1)
  }
  constructor(options = {}) {
    this.options = options
    this.cols = 80
    this.rows = 24
    this.written = [] // every chunk handed to `write`, as given
    this.disposed = false
    this.resets = 0
    this.focused = 0
    this.handlers = { data: [], binary: [] }
    Terminal.instances.push(this)
  }
  loadAddon(addon) {
    addon.terminal = this
  }
  open(element) {
    this.element = element
  }
  write(data) {
    this.written.push(data)
  }
  reset() {
    this.resets += 1
    this.written = []
  }
  focus() {
    this.focused += 1
  }
  dispose() {
    this.disposed = true
  }
  onData(fn) {
    this.handlers.data.push(fn)
    return { dispose: () => (this.handlers.data = this.handlers.data.filter((h) => h !== fn)) }
  }
  onBinary(fn) {
    this.handlers.binary.push(fn)
    return { dispose: () => (this.handlers.binary = this.handlers.binary.filter((h) => h !== fn)) }
  }
  /** A key press as xterm hands it over: a string. */
  type(text) {
    this.handlers.data.forEach((fn) => fn(text))
  }
}

export class FitAddon {
  static size = { cols: 100, rows: 30 } // what the next `fit()` measures
  static fits = 0
  fit() {
    FitAddon.fits += 1
    this.terminal.cols = FitAddon.size.cols
    this.terminal.rows = FitAddon.size.rows
  }
}

export class FakeSocket {
  static instances = []
  static get last() {
    return FakeSocket.instances.at(-1)
  }
  constructor(url) {
    this.url = url
    this.readyState = 0
    this.sent = [] // strings (text frames) and Uint8Arrays (binary frames), in order
    this.closed = false
    FakeSocket.instances.push(this)
  }
  send(data) {
    this.sent.push(data)
  }
  close() {
    this.closed = true
    this.readyState = 3
  }
  open() {
    this.readyState = 1
    act(() => this.onopen?.({}))
  }
  /** A frame from the server: an object is a text frame (JSON), bytes are a binary frame. */
  receive(frame) {
    const data = ArrayBuffer.isView(frame) ? frame.buffer.slice(frame.byteOffset, frame.byteOffset + frame.byteLength) : JSON.stringify(frame)
    act(() => this.onmessage?.({ data }))
  }
  drop() {
    this.readyState = 3
    act(() => this.onclose?.({ code: 1006 }))
  }
  get texts() {
    return this.sent.filter((s) => typeof s === 'string').map((s) => JSON.parse(s))
  }
  get binaries() {
    return this.sent.filter((s) => typeof s !== 'string')
  }
}
FakeSocket.CONNECTING = 0
FakeSocket.OPEN = 1
FakeSocket.CLOSING = 2
FakeSocket.CLOSED = 3

export class FakeResizeObserver {
  static instances = []
  constructor(fn) {
    this.fn = fn
    this.disconnected = false
    FakeResizeObserver.instances.push(this)
  }
  observe() {}
  disconnect() {
    this.disconnected = true
  }
  fire() {
    act(() => this.fn([]))
  }
}

export const resetFakes = () => {
  Terminal.instances = []
  FakeSocket.instances = []
  FakeResizeObserver.instances = []
  FitAddon.size = { cols: 100, rows: 30 }
  FitAddon.fits = 0
}
