#!/bin/bash

VIDEO_KID="eb676abbcb345e96bbcf616630f1a3da"
VIDEO_KEY="100b6c20940f779a4589152b57d2dacb"
AUDIO_KID="63cb5f7184dd4b689a5c5ff11ee6a328"
AUDIO_KEY="3bda3329158a4789880816a70e7e436d"

mkdir -p cbcs-multi
mkdir -p cbcs-single
mkdir -p cenc-multi
mkdir -p cenc-single

ffmpeg -y -f lavfi -i testsrc=duration=5:size=1920x1080:rate=24 \
       -f lavfi -i sine=frequency=440:duration=5 \
       -c:v libx264 -c:a aac -b:a 128k test.mp4

# ==========================================
# CENC MULTI-KEY
# ==========================================

packager \
  input=test.mp4,stream=video,init_segment=cenc-multi/video_init.mp4,segment_template='cenc-multi/video_$Number$.m4s',drm_label=VID_KEY \
  input=test.mp4,stream=audio,init_segment=cenc-multi/audio_init.mp4,segment_template='cenc-multi/audio_$Number$.m4s',drm_label=AUD_KEY \
  --enable_raw_key_encryption \
  --keys "label=VID_KEY:key_id=$VIDEO_KID:key=$VIDEO_KEY,label=AUD_KEY:key_id=$AUDIO_KID:key=$AUDIO_KEY" \
  --protection_scheme cenc \
  --clear_lead 0 \
  --segment_duration 10

# ==========================================
# CENC SINGLE-KEY
# ==========================================

packager \
  input=test.mp4,stream=video,init_segment=cenc-single/video_init.mp4,segment_template='cenc-single/video_$Number$.m4s',drm_label=SINGLE_KEY \
  input=test.mp4,stream=audio,init_segment=cenc-single/audio_init.mp4,segment_template='cenc-single/audio_$Number$.m4s',drm_label=SINGLE_KEY \
  --enable_raw_key_encryption \
  --keys "label=SINGLE_KEY:key_id=$VIDEO_KID:key=$VIDEO_KEY" \
  --protection_scheme cenc \
  --clear_lead 0 \
  --segment_duration 10

# ==========================================
# CBCS MULTI-KEY
# ==========================================

packager \
  input=test.mp4,stream=video,init_segment=cbcs-multi/video_init.mp4,segment_template='cbcs-multi/video_$Number$.m4s',drm_label=VID_KEY \
  input=test.mp4,stream=audio,init_segment=cbcs-multi/audio_init.mp4,segment_template='cbcs-multi/audio_$Number$.m4s',drm_label=AUD_KEY \
  --enable_raw_key_encryption \
  --keys "label=VID_KEY:key_id=$VIDEO_KID:key=$VIDEO_KEY,label=AUD_KEY:key_id=$AUDIO_KID:key=$AUDIO_KEY" \
  --protection_scheme cbcs \
  --clear_lead 0 \
  --segment_duration 10

# ==========================================
# CBCS SINGLE-KEY
# ==========================================

packager \
  input=test.mp4,stream=video,init_segment=cbcs-single/video_init.mp4,segment_template='cbcs-single/video_$Number$.m4s',drm_label=SINGLE_KEY \
  input=test.mp4,stream=audio,init_segment=cbcs-single/audio_init.mp4,segment_template='cbcs-single/audio_$Number$.m4s',drm_label=SINGLE_KEY \
  --enable_raw_key_encryption \
  --keys "label=SINGLE_KEY:key_id=$VIDEO_KID:key=$VIDEO_KEY" \
  --protection_scheme cbcs \
  --clear_lead 0 \
  --segment_duration 10

rm test.mp4
