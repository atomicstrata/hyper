// macOS animated README preview, using only system frameworks.
// swift scripts/encode_gif.swift INPUT.mp4 OUTPUT.gif WIDTH FPS
import Foundation
import AVFoundation
import ImageIO
import UniformTypeIdentifiers

func fail(_ message: String) -> Never {
    fputs(message + "\n", stderr)
    exit(1)
}
guard CommandLine.arguments.count == 5,
      let width = Int(CommandLine.arguments[3]), width > 0,
      let fps = Int(CommandLine.arguments[4]), (1...30).contains(fps) else {
    fail("Usage: swift scripts/encode_gif.swift INPUT.mp4 OUTPUT.gif WIDTH FPS (1–30)")
}
let source = AVURLAsset(url: URL(fileURLWithPath: CommandLine.arguments[1]))
let output = URL(fileURLWithPath: CommandLine.arguments[2])
guard !FileManager.default.fileExists(atPath: output.path) else { fail("Output already exists") }
let duration = source.duration.seconds
guard duration.isFinite && duration > 0,
      let track = source.tracks(withMediaType: .video).first else { fail("No valid video track") }
let transformed = track.naturalSize.applying(track.preferredTransform)
let height = Int((Double(width) * abs(transformed.height / transformed.width)).rounded())
let count = Int(ceil(duration * Double(fps)))
let generator = AVAssetImageGenerator(asset: source)
generator.appliesPreferredTrackTransform = true
generator.maximumSize = CGSize(width: width, height: height)
generator.requestedTimeToleranceBefore = .zero
generator.requestedTimeToleranceAfter = .zero
guard let destination = CGImageDestinationCreateWithURL(output as CFURL, UTType.gif.identifier as CFString, count, nil) else {
    fail("Cannot create GIF")
}
CGImageDestinationSetProperties(destination, [
    kCGImagePropertyGIFDictionary: [kCGImagePropertyGIFLoopCount: 0]
] as CFDictionary)
let gifDuration = (Double(count) * 100.0 / Double(fps)).rounded() / 100.0
var encodedWidth = 0, encodedHeight = 0
for frame in 0..<count {
    try autoreleasepool {
        let time = CMTime(seconds: Double(frame) / Double(fps), preferredTimescale: 600)
        let image = try generator.copyCGImage(at: time, actualTime: nil)
        encodedWidth = image.width
        encodedHeight = image.height
        // GIF delays have centisecond precision. Round cumulative boundaries,
        // not each delay, to avoid speeding up 12/24/30 fps clips on export.
        let start = (Double(frame) * 100.0 / Double(fps)).rounded()
        let end = (Double(frame + 1) * 100.0 / Double(fps)).rounded()
        let delay = (end - start) / 100.0
        let frameProperties = [kCGImagePropertyGIFDictionary: [
            kCGImagePropertyGIFDelayTime: delay,
            kCGImagePropertyGIFUnclampedDelayTime: delay
        ]] as CFDictionary
        CGImageDestinationAddImage(destination, image, frameProperties)
    }
}
guard CGImageDestinationFinalize(destination) else { fail("GIF encoding failed") }
print("Encoded \(count) frames, \(encodedWidth)×\(encodedHeight), \(gifDuration) seconds, infinite loop: \(output.path)")
