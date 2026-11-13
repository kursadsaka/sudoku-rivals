import Foundation
import Testing
@testable import SudokuRivals

private struct BoardVector: Decodable {
    let size: UInt8
    let uniquePuzzle: [UInt8]
    let solution: [UInt8]
    let invalidDuplicate: [UInt8]
}

private final class BundleMarker {}

private func vector(size: UInt8) throws -> BoardVector {
    let url = try #require(Bundle(for: BundleMarker.self).url(forResource: "boards", withExtension: "json"))
    let vectors = try JSONDecoder().decode([BoardVector].self, from: Data(contentsOf: url))
    return try #require(vectors.first { $0.size == size })
}

@Test("Native validateBoard accepts valid boards and rejects duplicates", arguments: [UInt8(6), 9])
func validateNativeBoard(size: UInt8) throws {
    let board = try vector(size: size)
    #expect(validateBoard(cells: Data(board.uniquePuzzle), size: size))
    #expect(validateBoard(cells: Data(board.solution), size: size))
    #expect(!validateBoard(cells: Data(board.invalidDuplicate), size: size))
    #expect(!validateBoard(cells: Data(board.solution.dropLast()), size: size))
}

@Test("Native solo bindings round-trip state and throw typed errors", arguments: [UInt8(6), 9])
func soloNativeBindings(size: UInt8) throws {
    let board = try vector(size: size)
    let saved = try newSolo(givens: Data(board.uniquePuzzle), size: size)
    #expect(try !soloIsComplete(saved: saved))
    let index = try #require(board.uniquePuzzle.firstIndex(of: 0))
    let updated = try updateSolo(saved: saved, command: "{\"type\":\"set\",\"index\":\(index),\"value\":\(board.solution[index])}")
    #expect(updated != saved)
    #expect(try !soloIsComplete(saved: updated))
    let complete = try newSolo(givens: Data(board.solution), size: size)
    #expect(try soloIsComplete(saved: complete))
    #expect(throws: SoloError.self) {
        try updateSolo(saved: "{}", command: "{}")
    }
}
