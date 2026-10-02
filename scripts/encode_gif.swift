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
let frameProperties = [kCGImagePropertyGIFDictionary: [
    kCGImagePropertyGIFDelayTime: 1.0 / Double(fps),
    kCGImagePropertyGIFUnclampedDelayTime: 1.0 / Double(fps)
]] as CFDictionary
for frame in 0..<count {
    try autoreleasepool {
        let time = CMTime(seconds: Double(frame) / Double(fps), preferredTimescale: 600)
        let image = try generator.copyCGImage(at: time, actualTime: nil)
        CGImageDestinationAddImage(destination, image, frameProperties)
    }
}
guard CGImageDestinationFinalize(destination) else { fail("GIF encoding failed") }
print("Encoded \(count) frames, \(width)×\(height), \(duration) seconds, infinite loop: \(output.path)")
