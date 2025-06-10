# Arcaea Hit Sound Generator
A mini sound generator that parses `.aff` file.

> **Credit to: [penguin-71630](https://penguin-71630.github.io/about/)**

## Usage
```bash
~\arcaea_auto_hitsound.exe sound -- <INPUT_PATH> <OUTPUT_PATH> <HIT_SOUND_PATH>
```
* `<INPUT_PATH>`: `.aff` file location
* `<OUTPUT_PATH>`: output `.wav` file location
* `<HIT_SOUND_PATH>`: hit sound `.wav` file location
> Example: `.\arcaea-auto-hitsound.exe sound -- D:\fragrance.aff D:\fragrance.wav "D:\hit_sound_tap.wav"`

## Roadmap

Current generator cannot parse complicated `timinggroups` correctly (such as `MEGALOVANIA (Camellia Remix)`). This may be augmented in the future.