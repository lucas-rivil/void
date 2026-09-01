// Preferred input (microphone) and output (speaker) devices, persisted in
// localStorage and shared across the recorder and the voice-note players.

const MIC_KEY = "void-mic";
const SPEAKER_KEY = "void-speaker";

export function getPreferredMic(): string | null {
  return localStorage.getItem(MIC_KEY) || null;
}

export function setPreferredMic(deviceId: string): void {
  if (deviceId) localStorage.setItem(MIC_KEY, deviceId);
  else localStorage.removeItem(MIC_KEY);
}

export function getPreferredSpeaker(): string | null {
  return localStorage.getItem(SPEAKER_KEY) || null;
}

export function setPreferredSpeaker(deviceId: string): void {
  if (deviceId) localStorage.setItem(SPEAKER_KEY, deviceId);
  else localStorage.removeItem(SPEAKER_KEY);
}

// Route an <audio> element to the user's preferred output device, if one is
// selected and the platform supports setSinkId (Chromium/WebView2 does).
export async function applyAudioSink(el: HTMLAudioElement | null): Promise<void> {
  if (!el) return;
  const sink = getPreferredSpeaker();
  if (!sink) return;
  if (typeof el.setSinkId !== "function") return;
  try {
    await el.setSinkId(sink);
  } catch {
    // device unplugged or not permitted — fall back to system default
  }
}
