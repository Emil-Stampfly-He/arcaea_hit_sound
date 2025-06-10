use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader},
    path::PathBuf,
    error::Error,
};
use hound::{SampleFormat, WavReader};
use regex::Regex;

struct AudioBuffer { 
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16, 
}

impl AudioBuffer {
    
    fn new_silence(duration_ms: u32, sample_rate: u32, channels: u16) -> Self {
        let total_samples =
            (duration_ms as u64 * sample_rate as u64 / 1000) * channels as u64;
        AudioBuffer {
            samples: vec![0.0; total_samples as usize],
            sample_rate,
            channels,
        }
    }
    
    fn from_wav(path: &str) -> Result<Self, Box<dyn Error>> {
        let mut reader = WavReader::open(path)?;
        let spec = reader.spec();

        // formats and bits per sample
        let samples: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
            // int PCM, <= 16
            (SampleFormat::Int, 1..=16) => reader
                .samples::<i16>()
                .map(|r| r.map(|i| i as f32 / i16::MAX as f32))
                .collect::<Result<Vec<f32>, hound::Error>>()?,

            // int PCM，>=16
            (SampleFormat::Int, 17..=32) => reader
                .samples::<i32>()
                .map(|r| r.map(|i| i as f32 / i32::MAX as f32))
                .collect::<Result<Vec<f32>, hound::Error>>()?,

            // float PCM，32
            (SampleFormat::Float, 32) => reader
                .samples::<f32>()
                .map(|r| r.map_err(Into::into))
                .collect::<Result<_, Box<dyn Error>>>()?,

            // other unsupported formats
            _ => {
                return Err(
                    format!(
                        "Unsupported WAV format: {:?}, {} bits",
                        spec.sample_format, spec.bits_per_sample
                    )
                        .into(),
                );
            }
        };

        Ok(AudioBuffer {
            samples,
            sample_rate: spec.sample_rate,
            channels: spec.channels,
        })
    }
    
    fn mix_at(&mut self, other: &AudioBuffer, at_ms: u32) { assert_eq!(self.sample_rate, other.sample_rate);
        assert_eq!(self.channels, other.channels);
        let start_idx = (at_ms as u64 * self.sample_rate as u64 / 1000
            * self.channels as u64) as usize;
        for (i, &s) in other.samples.iter().enumerate() {
            if start_idx + i < self.samples.len() {
                self.samples[start_idx + i] += s;
            }
        }
    }
    
    fn save_wav(&self, out: &PathBuf) -> Result<(), Box<dyn Error>> {
        let spec = hound::WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(out, spec)?;
        for &s in &self.samples {
            let clipped = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(clipped)?;
        }
        writer.finalize()?;
        Ok(())
    }
}

pub fn output(
    input_path: PathBuf, 
    output_path: PathBuf,
    hit_sound_path: PathBuf,
) -> Result<(), Box<dyn Error>> {
    let aff_file = OpenOptions::new().read(true).open(&input_path)?;
    let reader = BufReader::new(aff_file);
    
    // match arc(start_time, …, flag)
    let arc_re = Regex::new(
        r"^arc\(\s*(?P<start>\d+)\s*,[^)]*?,\s*(?P<flag>true|false)\s*\)"
    )?;
    // match arctap(tap_time);
    let arctap_re = Regex::new(r"arctap\(\s*(?P<tap>\d+)\s*\)")?;
    
    // match hold(start_time, end_time);
    let hold_re = Regex::new(r"^hold\(\s*(?P<start>\d+)\s*,")?;
    
    // match tap(start_time, …);
    let tap_re = Regex::new(r"^\(\s*(?P<time>\d+)\s*,")?;

    // get all hit times of hit_sound_sky.wav with millisecond as time unit
    let mut hit_times: Vec<u32> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        
        // 10. break once read "timinggroup"
        if line.starts_with("timinggroup") {
            break;
        }
        
        // 1–3. ignore all
        if line.starts_with("AudioOffset")
            || line.starts_with('-')
            || line.starts_with("timing") {
            continue;
        }

        // arc series
        if let Some(caps) = arc_re.captures(&line) {
            let start: u32 = caps["start"].parse().unwrap();
            let flag = &caps["flag"] == "true";

            // catch all arctaps in this arc
            let taps: Vec<u32> = arctap_re
                .captures_iter(&line)
                .map(|c| c["tap"].parse().unwrap())
                .collect();

            if !taps.is_empty() {
                // 4. & 5. with arctap: add it no matter flag
                hit_times.extend(&taps);
                // 5. flag == false, should arc start
                if !flag {
                    hit_times.push(start);
                }
            } else {
                // 6. & 7. without arctap：add arc start only if flag == false
                if !flag {
                    hit_times.push(start);
                }
            }
            continue;
        }

        // 8. hold
        if let Some(caps) = hold_re.captures(&line) {
            let start: u32 = caps["start"].parse().unwrap();
            hit_times.push(start);
            continue;
        }

        // 9. tap
        if let Some(caps) = tap_re.captures(&line) {
            let t: u32 = caps["time"].parse().unwrap();
            hit_times.push(t);
            continue;
        }
    }
    
    //hit_times.sort_unstable();
    
    let hit_sound = AudioBuffer::from_wav(hit_sound_path.to_str().unwrap())?;
    let max_hit = hit_times.iter().copied().max().unwrap_or(0);
    
    let sound_len_ms = (hit_sound.samples.len() as u64
        / hit_sound.sample_rate as u64
        / hit_sound.channels as u64)
        * 1000;
    let total_ms = max_hit as u64 + sound_len_ms;
    
    let mut output =
        AudioBuffer::new_silence(total_ms as u32, hit_sound.sample_rate, hit_sound.channels);

    for &t in &hit_times {
        output.mix_at(&hit_sound, t);
    }
    
    output.save_wav(&output_path)?;
    Ok(())
}
