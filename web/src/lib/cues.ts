export type Cue = 'join' | 'leave' | 'viewing'

const NOTES: Record<Cue, number[]> = {
  join: [523.25, 783.99],
  leave: [783.99, 523.25],
  viewing: [659.25, 659.25],
}

const NOTE_SECONDS = 0.09
const VOLUME = 0.08

let ctx: AudioContext | null = null

export function playCue(cue: Cue): void {
  try {
    const ac = (ctx ??= new AudioContext())
    if (ac.state === 'suspended') void ac.resume()
    const start = ac.currentTime + 0.01
    NOTES[cue].forEach((freq, i) => {
      const at = start + i * (NOTE_SECONDS + 0.03)
      const osc = ac.createOscillator()
      const gain = ac.createGain()
      osc.type = 'sine'
      osc.frequency.value = freq
      gain.gain.setValueAtTime(0, at)
      gain.gain.linearRampToValueAtTime(VOLUME, at + 0.01)
      gain.gain.exponentialRampToValueAtTime(0.0001, at + NOTE_SECONDS)
      osc.connect(gain).connect(ac.destination)
      osc.start(at)
      osc.stop(at + NOTE_SECONDS)
    })
  } catch {
    void 0
  }
}
