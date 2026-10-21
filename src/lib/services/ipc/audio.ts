import { invoke } from "@tauri-apps/api/core";
import type { AudioInputTestResult } from "../../types/recording.js";

export function testAudioInputs(
  microphoneEnabled: boolean,
  systemAudioEnabled: boolean,
  microphoneDeviceId?: string
): Promise<AudioInputTestResult> {
  return invoke<AudioInputTestResult>("test_audio_inputs", {
    microphoneEnabled,
    systemAudioEnabled,
    microphoneDeviceId
  });
}
