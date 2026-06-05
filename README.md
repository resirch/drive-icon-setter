# Drive Icon Setter

Change the icon for a drive in File Explorer on Windows 11 and Windows 10. Pick a drive letter, choose an image, and apply. To restore the default icon, remove the override.

Works with `.ico` files. You can also use `.png` files; the app converts them to `.ico` automatically and saves the converted file next to the original.

## Download

Download `drive-icon-setter.exe` from the [latest release](https://github.com/resirch/drive-icon-setter/releases/latest).

## Quick start

1. Run `drive-icon-setter.exe` (no command-line arguments).
2. Enter the drive letter (for example `F` or `F:`).
3. Browse for an `.ico` or `.png` file.
4. Choose who the change applies to:
   - **Current user** — only your account. No administrator rights needed.
   - **All users** — every account on the PC. Run the app as administrator.
5. Click **Apply icon**.

Close File Explorer completely, then open it again (Win+E) so the new icon appears.

## Remove a custom icon

1. Open the app.
2. Enter the same drive letter.
3. Choose the same scope you used when applying the icon.
4. Click **Remove override**.

Restart File Explorer again to see the default icon.

## Command line

The same options are available from a terminal:

```powershell
drive-icon-setter F C:\Icons\Backup.ico
drive-icon-setter F C:\Icons\Backup.png
drive-icon-setter F C:\Windows\Backup.ico --scope machine
drive-icon-setter F --remove
```

`--scope user` is the default. Use `--scope machine` for all users (requires an elevated terminal).

## PNG notes

If you pick a PNG, the app creates a sibling `.ico` file (for example `Backup.png` becomes `Backup.ico` in the same folder) and uses that file for the drive icon. Large images are resized to fit Windows icon limits.

## Build from source

Requires [Rust](https://www.rust-lang.org/tools/install).

```powershell
cargo build --release
```

The GUI opens when you run the built executable with no arguments:

```powershell
cargo run
```

## Publish a release

Push a version tag to build the Windows executable and attach it to a GitHub release:

```powershell
git tag v0.1.0
git push origin v0.1.0
```

You can also run the **Release** workflow manually from the Actions tab.
