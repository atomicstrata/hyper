// macOS encoder, no third-party dependencies.
// swift scripts/encode_video.swift FRAME_DIRECTORY OUTPUT.mp4 FPS
import Foundation
import AVFoundation
import AppKit

func fail(_ message: String) -> Never {
    fputs(message + "\n", stderr)
    exit(1)
}
guard CommandLine.arguments.count == 4,
      let fps = Int32(CommandLine.arguments[3]), fps > 0 else {
    fail("Usage: swift scripts/encode_video.swift FRAME_DIRECTORY OUTPUT.mp4 FPS")
}
let directory = URL(fileURLWithPath: CommandLine.arguments[1])
let output = URL(fileURLWithPath: CommandLine.arguments[2])
guard !FileManager.default.fileExists(atPath: output.path) else { fail("Output already exists") }
let files = try FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)
    .filter { $0.pathExtension == "png" }.sorted { $0.lastPathComponent < $1.lastPathComponent }
guard let first = files.first, let data = try? Data(contentsOf: first),
      let bitmap = NSBitmapImageRep(data: data) else { fail("No PNG frames") }
let width = bitmap.pixelsWide, height = bitmap.pixelsHigh
let writer = try AVAssetWriter(outputURL: output, fileType: .mp4)
writer.shouldOptimizeForNetworkUse = true
let input = AVAssetWriterInput(mediaType: .video, outputSettings: [
    AVVideoCodecKey: AVVideoCodecType.h264,
    AVVideoWidthKey: width, AVVideoHeightKey: height,
    AVVideoCompressionPropertiesKey: [AVVideoAverageBitRateKey: 6_000_000,
                                      AVVideoMaxKeyFrameIntervalKey: Int(fps) * 2]
])
input.expectsMediaDataInRealTime = false
let adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input, sourcePixelBufferAttributes: [
    kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32ARGB,
    kCVPixelBufferWidthKey as String: width, kCVPixelBufferHeightKey as String: height,
    kCVPixelBufferCGImageCompatibilityKey as String: true,
    kCVPixelBufferCGBitmapContextCompatibilityKey as String: true
])
writer.add(input)
guard writer.startWriting() else { fail("Start failed: \(String(describing: writer.error))") }
writer.startSession(atSourceTime: .zero)
for (index, file) in files.enumerated() {
    guard file.lastPathComponent == String(format: "%06d.png", index) else { fail("Missing or misnumbered frame at \(index)") }
    while !input.isReadyForMoreMediaData {
        if writer.status == .failed { fail("Encoding failed: \(String(describing: writer.error))") }
        Thread.sleep(forTimeInterval: 0.005)
    }
    autoreleasepool {
        guard let data = try? Data(contentsOf: file), let rep = NSBitmapImageRep(data: data), let cg = rep.cgImage,
              rep.pixelsWide == width, rep.pixelsHigh == height else { fail("Invalid frame: \(file.path)") }
        var buffer: CVPixelBuffer?
        guard let pool = adaptor.pixelBufferPool,
              CVPixelBufferPoolCreatePixelBuffer(nil, pool, &buffer) == kCVReturnSuccess,
              let pixel = buffer else { fail("Pixel buffer allocation failed") }
        CVPixelBufferLockBaseAddress(pixel, [])
        guard let context = CGContext(data: CVPixelBufferGetBaseAddress(pixel), width: width, height: height,
            bitsPerComponent: 8, bytesPerRow: CVPixelBufferGetBytesPerRow(pixel),
            space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.noneSkipFirst.rawValue) else {
            fail("Cannot create frame context")
        }
        context.draw(cg, in: CGRect(x: 0, y: 0, width: width, height: height))
        CVPixelBufferUnlockBaseAddress(pixel, [])
        guard adaptor.append(pixel, withPresentationTime: CMTime(value: Int64(index), timescale: fps)) else {
            fail("Frame append failed: \(String(describing: writer.error))")
        }
    }
}
input.markAsFinished()
writer.endSession(atSourceTime: CMTime(value: Int64(files.count), timescale: fps))
let done = DispatchSemaphore(value: 0)
writer.finishWriting { done.signal() }
done.wait()
guard writer.status == .completed else { fail("Finish failed: \(String(describing: writer.error))") }
print("Encoded \(files.count) frames, \(width)×\(height), \(fps) fps: \(output.path)")
