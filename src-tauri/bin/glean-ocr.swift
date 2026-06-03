import Foundation
import Vision
import AppKit

struct OCRResult: Codable {
    let text: String
    let confidence: Float
    let observations: [Observation]
}

struct Observation: Codable {
    let text: String
    let confidence: Float
    let x: Float
    let y: Float
    let width: Float
    let height: Float
}

func runOCR(path: String) -> OCRResult {
    let url = URL(fileURLWithPath: path)
    guard let image = NSImage(contentsOf: url),
          let cg = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
        return OCRResult(text: "", confidence: 0, observations: [])
    }

    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.recognitionLanguages = ["zh-Hans", "zh-Hant", "en-US", "ja-JP"]
    request.usesLanguageCorrection = true

    let handler = VNImageRequestHandler(cgImage: cg, options: [:])
    do {
        try handler.perform([request])
    } catch {
        return OCRResult(text: "", confidence: 0, observations: [])
    }

    let observations = (request.results ?? []).compactMap { obs -> Observation? in
        guard let candidate = obs.topCandidates(1).first else { return nil }
        let box = obs.boundingBox
        return Observation(
            text: candidate.string,
            confidence: candidate.confidence,
            x: Float(box.origin.x),
            y: Float(box.origin.y),
            width: Float(box.width),
            height: Float(box.height)
        )
    }

    let combined = observations
        .map { $0.text }
        .joined(separator: "\n")
    let avgConf = observations.isEmpty ? Float(0) : observations.map { $0.confidence }.reduce(0, +) / Float(observations.count)

    return OCRResult(text: combined, confidence: avgConf, observations: observations)
}

let args = CommandLine.arguments
guard args.count >= 2 else {
    let err = ["error": "usage: glean-ocr <image_path>"]
    let data = try! JSONSerialization.data(withJSONObject: err, options: [])
    FileHandle.standardError.write(data)
    exit(2)
}

let result = runOCR(path: args[1])
let encoder = JSONEncoder()
encoder.outputFormatting = [.withoutEscapingSlashes]
let data = try! encoder.encode(result)
print(String(data: data, encoding: .utf8) ?? "{}")
