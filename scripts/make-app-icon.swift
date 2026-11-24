// Flat brand/Windows rendering of docs/brand/rmux.svg using the existing SDK.
// macOS app bundles use rmux.icon via make-macos-icon.py instead.
// Keep this geometry and the SVG master in sync.
import AppKit

guard CommandLine.arguments.count == 2 else {
    fatalError("Usage: swift scripts/make-app-icon.swift <output.iconset>")
}
let output = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
let background = NSColor(srgbRed: 37/255, green: 35/255, blue: 46/255, alpha: 1)
let accent = NSColor(srgbRed: 185/255, green: 175/255, blue: 1, alpha: 1)
for size in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let pixels = size * scale
        let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: pixels,
            pixelsHigh: pixels, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
            isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
        let context = NSGraphicsContext(bitmapImageRep: bitmap)!
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = context
        let transform = AffineTransform(scale: CGFloat(pixels) / 128)
        (transform as NSAffineTransform).concat()
        background.setFill()
        NSBezierPath(roundedRect: NSRect(x: 4, y: 4, width: 120, height: 120),
                     xRadius: 28, yRadius: 28).fill()
        accent.setFill()
        for rect in [NSRect(x: 31, y: 32, width: 28, height: 64),
                     NSRect(x: 67, y: 68, width: 30, height: 28),
                     NSRect(x: 67, y: 32, width: 30, height: 28)] {
            NSBezierPath(rect: rect).fill()
        }
        background.setStroke()
        let prompt = NSBezierPath()
        prompt.move(to: NSPoint(x: 39, y: 79))
        prompt.line(to: NSPoint(x: 49, y: 64))
        prompt.line(to: NSPoint(x: 39, y: 49))
        prompt.lineWidth = 6
        prompt.lineCapStyle = .round
        prompt.lineJoinStyle = .round
        prompt.stroke()
        NSGraphicsContext.restoreGraphicsState()
        let suffix = scale == 2 ? "@2x" : ""
        try bitmap.representation(using: .png, properties: [:])!.write(
            to: output.appendingPathComponent("icon_\(size)x\(size)\(suffix).png"))
    }
}
