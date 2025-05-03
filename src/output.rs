#[allow(unused)]

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

        // 根据格式和位深做不同处理
        let samples: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
            // 整数 PCM，小于等于16位
            (SampleFormat::Int, 1..=16) => reader
                .samples::<i16>()
                .map(|r| r.map(|i| i as f32 / i16::MAX as f32))
                .collect::<Result<Vec<f32>, hound::Error>>()?,

            // 整数 PCM，大于16位（24位、32位）
            (SampleFormat::Int, 17..=32) => reader
                .samples::<i32>()
                .map(|r| r.map(|i| i as f32 / i32::MAX as f32))
                .collect::<Result<Vec<f32>, hound::Error>>()?,

            // 浮点 PCM，32位
            (SampleFormat::Float, 32) => reader
                .samples::<f32>()
                .map(|r| r.map_err(Into::into))
                .collect::<Result<_, Box<dyn Error>>>()?,

            // 其他格式暂不支持
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
    
    // arc(start_time, …, flag) [arctap(tap_time)];
    let arc_re = Regex::new(
        r"^arc\(\s*(?P<start>\d+)\s*,[^)]*?,\s*(?P<flag>true|false)\s*\)\s*(?P<tap>\[arctap\(\d+\)])?;"
    )?;
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
            
            // flag: if arc is blackline: true, else: false
            let flag = &caps["flag"] == "true";
            
            if let Some(arctap_str) = caps.name("tap") {
                // 4. & 5. arc with arctap
                let tap_caps = arctap_re.captures(arctap_str.as_str()).unwrap();
                let tap_time: u32 = tap_caps["tap"].parse().unwrap();
                // 4. & 5. add arctap hit sound
                hit_times.push(tap_time);
                if !flag {
                    // 5. flag == false, add arc start time
                    hit_times.push(start);
                }
            } else {
                // 6. & 7. arc without arctap
                if !flag {
                    // 7：flag == false, add arc start time
                    hit_times.push(start);
                }
                
                // 6. flag == true without arctap, ignore it
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
