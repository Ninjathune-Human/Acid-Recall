# Only used on the disposable Windows CI runner to capture the full desktop rack.
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class OpenRBCiDisplay {
    [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Ansi)]
    public struct Mode {
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst=32)] public string device;
        public short spec, driver, size, extra;
        public int fields, x, y, orientation, fixedOutput;
        public short color, duplex, yResolution, ttOption, collate;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst=32)] public string form;
        public short logPixels;
        public int bits, width, height, flags, frequency, icmMethod, icmIntent;
        public int media, dither, reserved1, reserved2, panningWidth, panningHeight;
    }
    [DllImport("user32.dll", CharSet=CharSet.Ansi)]
    public static extern bool EnumDisplaySettings(string device, int index, ref Mode mode);
    [DllImport("user32.dll", CharSet=CharSet.Ansi)]
    public static extern int ChangeDisplaySettings(ref Mode mode, int flags);
}
"@
$captureMode = New-Object OpenRBCiDisplay+Mode
$captureMode.size = [Runtime.InteropServices.Marshal]::SizeOf($captureMode)
if ([OpenRBCiDisplay]::EnumDisplaySettings($null, -1, [ref]$captureMode)) {
    foreach ($captureSize in @(@(1920, 1200), @(1920, 1080), @(1600, 1200))) {
        $captureMode.width = $captureSize[0]
        $captureMode.height = $captureSize[1]
        $captureMode.fields = 0x180000
        if ([OpenRBCiDisplay]::ChangeDisplaySettings([ref]$captureMode, 0) -eq 0) {
            Write-Output ("CI display: {0} x {1}" -f $captureMode.width, $captureMode.height)
            break
        }
    }
}
