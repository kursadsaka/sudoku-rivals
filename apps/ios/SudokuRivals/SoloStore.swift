import Foundation

/// Platform envelope around the core's versioned save. Rules and board
/// invariants stay in Rust; this only adds presentation state.
struct SoloEnvelope: Codable, Equatable {
    static let currentVersion = 1
    var version = SoloEnvelope.currentVersion
    var game: String
    var elapsedMs: Int
    var paused: Bool
    var selected: Int
    var notesMode: Bool
}

enum SlotStatus: Equatable {
    case missing
    case ready(SoloEnvelope, complete: Bool)
    /// Written by a newer (or unknown) format. Kept on disk untouched.
    case unsupportedVersion
    /// Unreadable or failing core validation. Kept on disk untouched.
    case corrupt
    /// The file exists but could not be read (I/O, protection).
    case unavailable

    var exists: Bool { self != .missing }
    var isReadable: Bool { if case .ready = self { true } else { false } }
}

/// One file per board size, written atomically (temp file + rename), so a
/// failed write leaves the previous save intact.
struct SoloStore {
    static let sizes = [6, 9]
    let directory: URL
    var failWrites = false

    static var standard: SoloStore {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return SoloStore(directory: base.appending(path: "SoloSaves", directoryHint: .isDirectory))
    }

    func url(_ size: Int) -> URL { directory.appending(path: "solo-\(size).json") }

    func read(_ size: Int) -> SlotStatus {
        let data: Data
        do { data = try Data(contentsOf: url(size)) }
        catch let error as CocoaError where error.code == .fileReadNoSuchFile { return .missing }
        catch { return .unavailable }
        return Self.decode(data, size: size)
    }

    static func decode(_ data: Data, size: Int) -> SlotStatus {
        struct Version: Decodable { let version: Int }
        // Core caps the inner game at 1 MiB; the envelope adds little.
        guard data.count <= 2 << 20,
              let version = try? JSONDecoder().decode(Version.self, from: data) else { return .corrupt }
        guard version.version == SoloEnvelope.currentVersion else {
            return version.version > SoloEnvelope.currentVersion ? .unsupportedVersion : .corrupt
        }
        guard let envelope = try? JSONDecoder().decode(SoloEnvelope.self, from: data),
              envelope.elapsedMs >= 0,
              (0..<size * size).contains(envelope.selected) else { return .corrupt }
        do {
            let complete = try soloIsComplete(saved: envelope.game)
            guard try BoardState(json: envelope.game).size == size else { return .corrupt }
            return .ready(envelope, complete: complete)
        } catch let SoloError.InvalidInput(reason) where reason == "UnsupportedVersion" {
            return .unsupportedVersion
        } catch {
            return .corrupt
        }
    }

    func write(_ envelope: SoloEnvelope, size: Int) throws {
        if failWrites { throw CocoaError(.fileWriteUnknown) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try JSONEncoder().encode(envelope).write(to: url(size), options: .atomic)
    }

    /// Moves an unreadable save aside (never deletes it) before a confirmed
    /// new game takes the slot. Throws if it cannot be preserved.
    @discardableResult
    func archiveUnreadable(_ size: Int, now: Date = .now) throws -> URL {
        let stamp = Int(now.timeIntervalSince1970 * 1000)
        let target = directory.appending(path: "solo-\(size).unreadable-\(stamp).json")
        try FileManager.default.moveItem(at: url(size), to: target)
        return target
    }
}
