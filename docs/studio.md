# OpenRB-338 Studio

## Pattern styles

The generator offers **Techno | IDM | Break**, shared by both TBs and both TRs.

- Techno: quarter-note kicks, offbeat hats, repeating bass motifs and interlocking percussion.
- IDM: irregular pulse distribution, metallic details, ghost notes and wider TB register changes.
- Break: syncopated kicks, backbeat snares and broken-beat / drum-and-bass variations.

These are musical grammars, not copies of any artist's tracks. The selected key and scale,
pattern length, LINK TB harmony, accents, slides and Undo Random continue to work.
The TR patterns retain the original machine's 1–16-step limit.

## SPACE — stereo convolution reverb

Click **SPACE** in the right FX column, or Options > SPACE / Convolution Reverb.
Turn up one or more machine sends. The **REV** button in each mixer strip toggles its send;
SPACE provides its exact amount. Sampler lanes have their own sends.

SPACE convolves the audio with a stereo impulse response. Four original **synthetic** spaces
are available: Tight room, Concrete chamber, Warehouse and Diffuse plate. Choose a space and
a duration, then **Build space**. These are generated IRs, not recordings of real rooms or
third-party presets. **Load IR** accepts mono/stereo WAV or uncompressed AIFF responses up
to six seconds. Imported IRs are energy-normalized.

Controls: Return, Pre-delay (0–200 ms), Stereo width, Low cut and High cut. Sends are
post-insert, post-sidechain and post-fader. Turning a send down lets the existing tail decay.
The return is added before the master compressor and master fader. IR changes crossfade;
rapid requests keep the latest prepared response queued behind the current fade.

PCF, Delay, Distortion and Compressor retain their existing sound and routing. Click their
cards to edit all their controls, including PCF faders/pattern/mode and Delay steps/triplet.

## Sampler — eight lanes, up to 64 bars per lane

Click **SAMPLER**, or Options > Sampler / 64-bar sequencer.

1. Choose a lane and click **Load**. WAV PCM 8/16/24/32-bit, WAV float 32/64-bit,
   uncompressed AIFF and MP3 are supported, mono or stereo. Imports are resampled to 44.1 kHz.
2. Choose **Length**, from 1 to 64 bars. Each bar contains sixteen steps: 1,024 steps at
   the maximum length. The eight lanes can have different lengths.
3. Click a step to toggle it; right-click to edit velocity (0–127).
4. Choose a bar in the overview. Disable Follow to edit another bar during playback.
   Copy/Paste/Clear operate on the selected lane's current bar; Repeat fills its chosen length.
5. Press Play in the sampler or on the main transport. Playback uses the same engine clock.

Modes:

- **One Shot:** plays the selected region to its end; up to four overlapping voices per lane.
- **Auto Cut:** a new trigger cuts that lane's previous voice with a short fade.
- **Gate:** releases after the chosen duration, expressed in sixteenth-note steps.
- **Loop:** repeats the selected region until retrigger, choke or Stop. Short seam fades
  reduce clicks while retaining the region's duration.

Each lane has Mute, Solo, Level, Pan, Pitch (±24 semitones), Reverse, Start/End trim,
Attack, Release, Gate and a reverb send. Pitch uses **varispeed**: it changes pitch and
duration together, not a phase-vocoder or tempo-warp algorithm. Choke 0 is independent;
lanes in the same group (1–8) cut each other. Audition works with the transport stopped;
Stop also ends an auditioned loop.

Limits: 180 seconds per imported sample; 240 MiB of decoded stereo sample audio per bank.
MP3 files are limited to 32 MiB and 180 seconds. Compressed AIFF/AIFC and multichannel audio
are not supported. Audio-file decoding,
resampling and IR preparation run in the background; their failure leaves existing audio intact.

## MIDI

CH and OH are independent in the desktop app: both notes can occupy the same step,
neither voice chokes the other, and the 909 closed-hat level stays separate from OH.
This applies to playback and desktop audio export; the standalone legacy renderer retains
its original compatibility mode unless independence is explicitly enabled.

The existing MIDI Learn and pickup work with the new controls:

- SPACE: Return, Pre-delay, Stereo width, Low cut, High cut and four machine sends.
- Sampler: Level, Pan, Pitch and Reverb send for each of the eight lanes.

Search **SPACE** or **Sampler / Lane N** in MIDI Controller. Existing mappings keep their IDs.
The UC-33 minimum-one calibration applies to the new knobs as well.

## Saving and exporting

Once Studio content is used, **Save / Save As** writes a portable **.orb338** project. It
embeds the original RBS song, samples, IR, all sampler steps, settings, sidechain and
independent closed-hat level. Open it through File > Open. The versioned container validates
section sizes and checksums before replacing the current song. A standard RBS opened and
edited without Studio content continues to save as RBS.

MIDI routing remains an application/controller preset; use MIDI Import/Export to move it.
Custom RBM drum kits remain external, as with ordinary RBS songs.

**Export Loop as Audio File** includes Studio's sampler and reverb with the same DSP as
live playback. Set the main song loop to the desired export length, e.g. 64 bars, and use
song mode as required by the existing export command. Audio export ends at that loop's end;
it does not append an extra reverb tail.

## Implementation and validation

The engine uses nonuniform FFT partitioned FIR convolution: the first 2,048 taps have
64-frame partitions, and the tail has 2,048-frame partitions. It adds no DSP lookahead delay.
The existing audio device's buffering still applies. Plans, spectra and sample buffers are
prepared off the audio thread; replaced resources return to the UI for destruction.

Sampler playback reads PCM directly at its original pitch and uses a precomputed 16-tap
bandlimited interpolation table for transposition, fixed voice
pools, smooth level/pan changes and bounded cut fades. Step crossings are resolved within
each 64-frame engine block. The original machines retain their original block scheduling.

Waveform caches are owned by the nine fixed display slots, each at most 2,048 peaks. DSP
ramps and filters discard inaudible residuals before they become subnormal values. The
convolution processor sleeps after its maximum possible tail has fully drained. Regression
checks include two minutes of playback/retrigger/silence and bounded repeated waveform updates.

MP3 decoding uses Symphonia 0.5.5 (MPL-2.0), entirely on the import worker. Source and license:
https://github.com/pdeljanov/Symphonia/tree/v0.5.5. The portable package includes its license.

Tests cover direct-FIR agreement across partition/history boundaries, audio import bounds,
sampler modes/chokes/stop, the last step of bar 64, precise step crossings, portable project
integrity, genre/harmony constraints and the real player's allocation/free guard.
