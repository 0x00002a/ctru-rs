#![feature(allocator_api)]

use ctru::linear::LinearAllocator;
use ctru::prelude::*;
use ctru::services::ndsp::{
    AudioFormat, InterpolationType, Ndsp, OutputMode, WaveBuffer, WaveInfo,
};

const SAMPLERATE: u32 = 22050;
const SAMPLESPERBUF: u32 = SAMPLERATE / 30; // 735
const BYTESPERSAMPLE: u32 = 4;

fn array_size(array: &[u8]) -> usize {
    array.len()
} // (sizeof(array)/sizeof(array[0]))

// audioBuffer is stereo PCM16
fn fill_buffer(audioData: &mut Box<[u8], LinearAlloc>, frequency: i32) {
    for i in 0..size {
        // This is a simple sine wave, with a frequency of `frequency` Hz, and an amplitude 30% of maximum.
        let sample: i16 = 0.3 * 0x7FFF * sin(frequency * (2 * std::f32::PI) * i / SAMPLERATE);

        // Stereo samples are interleaved: left and right channels.
        audioData[i] = (sample << 16) | (sample & 0xffff);
    }
}

fn main() {
    ctru::init();
    let gfx = Gfx::init().expect("Couldn't obtain GFX controller");
    let hid = Hid::init().expect("Couldn't obtain HID controller");
    let apt = Apt::init().expect("Couldn't obtain APT controller");
    let _console = Console::init(gfx.top_screen.borrow_mut());

    println!("libctru filtered streamed audio\n");

    let audioBuffer = Box::new_in(
        [0u32; (SAMPLESPERBUF * BYTESPERSAMPLE * 2)],
        LinearAllocator,
    );
    fill_buffer(audioBuffer, notefreq[note]);

    let audioBuffer1 =
        WaveBuffer::new(audioBuffer, AudioFormat::PCM16Stereo).expect("Couldn't sync DSP cache");
    let audioBuffer2 = audioBuffer1.clone();

    let fillBlock = false;

    let ndsp = Ndsp::init().expect("Couldn't obtain NDSP controller");

    // This line isn't needed since the default NDSP configuration already sets the output mode to `Stereo`
    ndsp.set_output_mode(OutputMode::Stereo);

    let channel_zero = ndsp.channel(0);
    channel_zero.set_interpolation(InterpolationType::Linear);
    channel_zero.set_sample_rate(SAMPLERATE);
    channel_zero.set_format(NDSP_FORMAT_STEREO_PCM16);

    // Output at 100% on the first pair of left and right channels.

    let mix = [0f32; 12];
    mix[0] = 1.0;
    mix[1] = 1.0;
    channel_zero.set_mix(mix);

    // Note Frequencies

    let notefreq = [
        220, 440, 880, 1760, 3520, 7040, 14080, 7040, 3520, 1760, 880, 440,
    ];

    let note: i32 = 4;

    // Filters

    let filter_names = [
        "None",
        "Low-Pass",
        "High-Pass",
        "Band-Pass",
        "Notch",
        "Peaking",
    ];

    let filter = 0;

    // We set up two wave buffers and alternate between the two,
    // effectively streaming an infinitely long sine wave.

    let mut buf1 = WaveInfo::new(&mut audioBuffer1, false);
    let mut buf2 = WaveInfo::new(&mut audioBuffer2, false);

    unsafe {
        channel_zero.add_wave_buffer(buf1);
        channel_zero.add_wave_buffer(buf2);
    };

    println!("Press up/down to change tone frequency\n");
    println!("Press left/right to change filter\n");
    println!("\x1b[6;1Hnote = {} Hz        ", notefreq[note]);
    println!("\x1b[7;1Hfilter = {}         ", filter_names[filter]);

    while apt.main_loop() {
        hid.scan_input();
        let keys_down = hid.keys_down();

        if keys_down.contains(KeyPad::KEY_START) {
            break;
        } // break in order to return to hbmenu

        if keys_down.contains(KeyPad::KEY_DOWN) {
            note -= 1;
            if (note < 0) {
                note = notefreq.len() - 1;
            }
            println!("\x1b[6;1Hnote = {} Hz        ", notefreq[note]);
        } else if keys_down.contains(KeyPad::KEY_UP) {
            note += 1;
            if (note >= notefreq.len()) {
                note = 0;
            }
            println!("\x1b[6;1Hnote = {} Hz        ", notefreq[note]);
        }

        let update_params = false;
        if keys_down.contains(KeyPad::KEY_LEFT) {
            filter -= 1;
            if (filter < 0) {
                filter = filter_names.len() - 1;
            }
            update_params = true;
        } else if keys_down.contains(KeyPad::KEY_LEFT) {
            filter += 1;
            if (filter >= filter_names.len()) {
                filter = 0;
            }
            update_params = true;
        }

        if update_params {
            println!("\x1b[7;1Hfilter = {}         ", filter_names[filter]);
            match filter {
                1 => ndspChnIirBiquadSetParamsLowPassFilter(0, 1760., 0.707),
                2 => ndspChnIirBiquadSetParamsHighPassFilter(0, 1760., 0.707),
                3 => ndspChnIirBiquadSetParamsBandPassFilter(0, 1760., 0.707),
                4 => ndspChnIirBiquadSetParamsNotchFilter(0, 1760., 0.707),
                5 => ndspChnIirBiquadSetParamsPeakingEqualizer(0, 1760., 0.707, 3.0),
                _ => ndspChnIirBiquadSetEnable(0, false),
            }
        }

        if waveBuf[fillBlock].status == NDSP_WBUF_DONE {
            if fillBlock {
                fill_buffer(buf1.data_pcm16, notefreq[note]);
                channel_zero.add_wave_buffer(buf1);
            } else {
                fill_buffer(waveBuf[fillBlock].data_pcm16, notefreq[note]);
                channel_zero.add_wave_buffer(buf2);
            }
            fillBlock = !fillBlock;
        }

        // Flush and swap framebuffers
        gfx.flush_buffers();
        gfx.swap_buffers();

        //Wait for VBlank
        gfx.wait_for_vblank();
    }
}
