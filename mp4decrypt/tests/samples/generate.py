import os
import subprocess

VIDEO_KID = "eb676abbcb345e96bbcf616630f1a3da"
VIDEO_KEY = "100b6c20940f779a4589152b57d2dacb"
AUDIO_KID = "63cb5f7184dd4b689a5c5ff11ee6a328"
AUDIO_KEY = "3bda3329158a4789880816a70e7e436d"

def run_command(cmd):
    try:
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except subprocess.CalledProcessError as e:
        print(f"Error running command: {e}")
        exit(1)

def run_packager(scheme, mode):
    output_dir = f"{scheme}-{mode}"
    print(f"Processing {output_dir}")

    match mode:
        case "multi":
            v_label = "VID_KEY"
            a_label = "AUD_KEY"
            keys = f"label={v_label}:key_id={VIDEO_KID}:key={VIDEO_KEY},label={a_label}:key_id={AUDIO_KID}:key={AUDIO_KEY}"
        case "single":
            v_label = "SINGLE_KEY"
            a_label = "SINGLE_KEY"
            keys = f"label={v_label}:key_id={VIDEO_KID}:key={VIDEO_KEY}"

    cmd = [
        "packager",
        f"input=test.mp4,stream=video,init_segment={output_dir}/video_init.mp4,segment_template={output_dir}/video_$Number$.m4s,drm_label={v_label}",
        f"input=test.mp4,stream=audio,init_segment={output_dir}/audio_init.mp4,segment_template={output_dir}/audio_$Number$.m4s,drm_label={a_label}",
        "--clear_lead", "0",
        "--enable_raw_key_encryption",
        "--keys", keys,
        "--protection_scheme", scheme,
        "--segment_duration", "10"
    ]

    run_command(cmd)

def main():
    print("Generating test.mp4")

    ffmpeg_cmd = [
        "ffmpeg", "-hide_banner", "-y",
        "-f", "lavfi", "-i", "testsrc=duration=5:size=1920x1080:rate=24",
        "-f", "lavfi", "-i", "sine=frequency=440:duration=5",
        "-c:v", "libx264", "-c:a", "aac", "-b:a", "128k", "test.mp4"
    ]
    run_command(ffmpeg_cmd)

    schemes = ["cenc", "cens", "cbc1", "cbcs"]
    modes = ["multi", "single"]

    for scheme in schemes:
        for mode in modes:
            run_packager(scheme, mode)

    if os.path.exists("test.mp4"):
        os.remove("test.mp4")

if __name__ == "__main__":
    main()