---
icon: fontawesome/brands/android
---

# Android Support

1. Install the [Termux](https://f-droid.org/en/packages/com.termux) app on your device, then enable storage permissions manually from its settings page. After that, run the following commands in the terminal.

    ```bash
    pkg update
    pkg upgrade
    pkg install ffmpeg
    ln -s /storage/emulated/0/Download Download
    ```

2. Install [vsd on termux](https://github.com/clitic/vsd/blob/main/vsd/BUILD.md#android-on-termux). Currently, only *arm64-v8a* binaries pre-builts are available which can be installed using the following command.

    ```bash
    curl -L https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-aarch64-linux-android.tar.xz | tar xJC $PREFIX/bin
    ```

3. Use third party browsers like [Kiwi Browser](https://github.com/kiwibrowser/src.next) (*developer tools*) paired with [Get cookies.txt LOCALLY](https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc) extension or [Via Browser](https://play.google.com/store/apps/details?id=mark.via.gp) (*tools > resource sniffer*) to find playlists within websites.

4. Now you can run vsd as usual. The streams would be directly downloaded in your android downloads folder.  

    ```bash
    cd Download
    vsd save <url> -o video.mp4
    ```
