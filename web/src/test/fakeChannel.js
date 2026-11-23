// T135: a stand-in for BroadcastChannel. Every instance with the same name gets what another posts (not the poster itself), at once, inside act.
import { act } from '@testing-library/react'

export class FakeChannel {
  static instances = []
  static sent = [] // every message posted, in order
  constructor(name) {
    this.name = name
    this.closed = false
    FakeChannel.instances.push(this)
  }
  postMessage(data) {
    FakeChannel.sent.push(data)
    for (const other of FakeChannel.instances) if (other !== this && !other.closed && other.name === this.name) act(() => other.onmessage?.({ data }))
  }
  close() {
    this.closed = true
  }
  /** A message from another window (a page this test does not render). */
  static from(data) {
    new FakeChannel('easy-personal-term').postMessage(data)
  }
}
export const resetChannels = () => {
  FakeChannel.instances = []
  FakeChannel.sent = []
}
