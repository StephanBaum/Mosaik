# Try Mosaic: a preview install

This guide installs a **preview build** of Mosaic on a Windows PC (the desktop app and the Chrome or Edge
extension) and on an Android phone. A preview is for trying the app out. Read "What a preview is not" at the end
before you put anything real in it.

The builds come from the `Preview` workflow (`.github/workflows/preview.yml`). It runs when a `preview/*` branch
is pushed, and you can start it by hand from the Actions tab on such a branch. Nothing in it uses a secret.

## 1. Download

1. Open the repository on GitHub, then **Actions**, then the latest **Preview** run.
2. Scroll to **Artifacts** at the bottom of the run page. Download the ones you need (GitHub needs you to be
   signed in):
   - `mosaic-preview-windows`: the Windows installer (`Mosaic-PREVIEW-UNSIGNED-<commit>-x64-setup.exe`)
   - `mosaic-preview-extension-chromium`: the extension for Chrome and Edge
   - `mosaic-preview-android`: the Android app (`Mosaic-PREVIEW-android-<commit>.apk`)
3. Each artifact is a zip. Unzip it. Each one holds a `SHA256SUMS` file next to its files.

If the run has no `mosaic-preview-android` artifact, the branch had no Android app yet.

## 2. Check the sums

A checksum proves the file you downloaded is the one the workflow made. In PowerShell, in the unzipped folder:

```powershell
Get-FileHash .\Mosaic-PREVIEW-UNSIGNED-*-x64-setup.exe -Algorithm SHA256
Get-Content .\SHA256SUMS
```

The long hex strings must be the same. Do this for each file. (On Linux or macOS: `sha256sum -c SHA256SUMS`.)
This protects against a damaged or swapped download. It does not replace a signature: the checksum file comes from
the same place as the file.

## 3. Windows: install the app

1. Run `Mosaic-PREVIEW-UNSIGNED-...-x64-setup.exe`. It installs for your user only (no administrator rights) into
   `%LOCALAPPDATA%\Mosaic`.
2. Windows SmartScreen will warn ("Windows protected your PC"), because the build is **not signed**. Choose
   **More info**, then **Run anyway**. This is expected for a preview.
3. Start Mosaic. The window title says "Mosaic (Preview / Vorschau)" and Settings shows "Preview build, not a
   release". Create a new account with a **test password**; do not reuse a real one.

## 4. The browser extension (Chrome or Edge)

1. Unzip `mosaic-preview-extension-chromium.zip` into a folder you will keep (the browser reads it from there).
2. Open `chrome://extensions` (Edge: `edge://extensions`). Switch on **Developer mode**. Choose **Load unpacked**
   and pick the unzipped folder (the one that holds `manifest.json`). Its ID will be
   `lccdgfpfnmegbdgooanfejdbahhokikm`. If the ID is different, the extension will not connect.
3. **The browser connection.** The desktop app writes the connection to your browser (the "native messaging"
   entry) every time it starts, for Chrome, Edge, Chromium and Firefox. Start Mosaic once, then **restart the
   browser**. If the popup still says "Mosaic isn't set up for this browser", write the entry by hand (PowerShell):

   ```powershell
   & "$env:LOCALAPPDATA\Mosaic\mosaic-native-host.exe" install
   ```

   then restart the browser. (`mosaic-native-host.exe uninstall` removes it again.) See
   [`NATIVE_MESSAGING.md`](NATIVE_MESSAGING.md) section 7.
4. **Pair the browser.** Keep the Mosaic app open. Click the Mosaic icon in the browser toolbar (pin it from the
   puzzle-piece menu first), choose **Connect**, and check that the app shows the same code. In the app click
   **Allow**. Then unlock the vault in the app; the extension fills one login after you click it.

The preview app accepts any unpacked extension that carries this key (see the ADR below). That is why the preview
is for trying out only.

## 5. Android: install the app

1. Copy the `.apk` to the phone (or download the artifact on the phone).
2. Open it. Android asks to allow installing apps from this source (your browser or file manager): allow it for
   this one install, then **Install**.
3. **The signing key is throwaway.** The workflow made it for this run and discarded it. The real release will be
   signed with a different key, and Android will not update over an app with a different key. Later you will have
   to **uninstall the preview first**. That deletes the phone's local data. Your vault is not lost: it stays on
   your other devices and on the sync server, and you set the phone up again.

## 6. Join the phone to your account

A phone joins an account that syncs through a Mosaic server (cloud-folder sync is not supported on phones yet).

1. On the desktop open **Settings**, then **Sync**, and choose **Set up a phone**. A QR code appears.
2. On the phone open Mosaic and choose to set up with a code. Hold the camera over the QR code.
3. Enter your master password and your Secret Key (from your Emergency Kit) when asked.

## 7. What a preview is not

- **Not a release.** It is unsigned on Windows and signed with a throwaway key on Android. Windows will warn you.
- **Pre-audit.** Mosaic has had no external security audit. Do not put real
  passwords in it.
- **Temporary.** The downloads disappear after **7 days**. Build again from the Actions tab when you need them.
- **Not the verified path.** Releases are signed with a published key and checked as described in
  [`VERIFY.md`](VERIFY.md). A preview has none of that.

The decision behind the preview build (it lets the desktop trust the dev extension ID) and its risk are written
down in ADR-0153 in [`DECISIONS.md`](DECISIONS.md) and as R153 in [`THREAT_MODEL.md`](THREAT_MODEL.md).
