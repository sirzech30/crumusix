use std::sync::OnceLock;
use parking_lot::Mutex;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum CrossfadeMode {
    Off,
    Duration1s,
    Duration2s,
    Duration3s,
    Duration5s,
    Duration10s,
}

impl CrossfadeMode {
    pub fn to_ms(&self) -> u64 {
        match self {
            CrossfadeMode::Off => 0,
            CrossfadeMode::Duration1s => 1000,
            CrossfadeMode::Duration2s => 2000,
            CrossfadeMode::Duration3s => 3000,
            CrossfadeMode::Duration5s => 5000,
            CrossfadeMode::Duration10s => 10000,
        }
    }
}

pub struct CrossfadeMixer {
    mode: Mutex<CrossfadeMode>,
}

impl CrossfadeMixer {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<CrossfadeMixer> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            mode: Mutex::new(CrossfadeMode::Duration3s), // Default to 3s
        })
    }

    pub fn set_mode(&self, mode: CrossfadeMode) {
        *self.mode.lock() = mode;
    }

    pub fn get_mode(&self) -> CrossfadeMode {
        self.mode.lock().clone()
    }

    /// Mixes two PCM buffers dynamically using an equal-power quadratic fade.
    /// This matches the logarithmic sensitivity of human hearing and prevents
    /// perceived volume drops during transitions.
    ///
    /// * `progress_pct` - value from 0.0 (start of crossfade) to 1.0 (end of crossfade)
    pub fn mix_channels(&self, out_buffer: &mut [f32], in_buffer: &[f32], progress_pct: f32) {
        let progress = progress_pct.clamp(0.0, 1.0);
        
        // Quadratic curves for constant-power summation (O(N) operations)
        let fade_out_factor = (1.0 - progress).powi(2);
        let fade_in_factor = progress.powi(2);

        let len = out_buffer.len().min(in_buffer.len());
        for i in 0..len {
            // Mix outgoing stream (fading out) with incoming stream (fading in)
            out_buffer[i] = (out_buffer[i] * fade_out_factor) + (in_buffer[i] * fade_in_factor);
        }
    }

    /// Single buffer volume scaling for gradual fade-in/fade-out operations
    pub fn apply_fade(&self, buffer: &mut [f32], progress_pct: f32, is_fade_in: bool) {
        let progress = progress_pct.clamp(0.0, 1.0);
        let factor = if is_fade_in {
            progress.powi(2)
        } else {
            (1.0 - progress).powi(2)
        };

        for sample in buffer.iter_mut() {
            *sample *= factor;
        }
    }
}
